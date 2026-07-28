use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use crate::db::conversations;
use crate::db::memories;
use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::llm::traits::{ChatCompletionMessage, ChatOptions, LlmProvider, StreamEvent};
use crate::models::chat::Message;
use crate::models::memory::{ExtractedMemory, Memory};
use crate::services::agent_engine;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

const LLM_EXTRACTION_TIMEOUT_SECONDS: u64 = 60;
const MAX_EXTRACTION_RESPONSE_BYTES: usize = 128 * 1024;
const MIN_COMPLETE_USER_MESSAGES: usize = 3;
const ALLOWED_CATEGORIES: &[&str] = &["preference", "task_status", "cognition_update", "fact"];

pub async fn extract_for_conversation(
    main_pool: DbPool,
    conv_pool: ConversationsPool,
    conversation_id: String,
) -> Result<usize, AppError> {
    match extract_for_conversation_inner(&main_pool, &conv_pool, &conversation_id).await {
        Ok(count) => Ok(count),
        Err(err) => {
            tracing::warn!(conversation_id, error = %err, "memory extraction skipped");
            Ok(0)
        }
    }
}

async fn extract_for_conversation_inner(
    main_pool: &DbPool,
    conv_pool: &ConversationsPool,
    conversation_id: &str,
) -> Result<usize, AppError> {
    let provider = agent_engine::resolve_default_provider(main_pool).await?;
    extract_for_conversation_with_provider(main_pool, conv_pool, conversation_id, provider).await
}

async fn extract_for_conversation_with_provider(
    main_pool: &DbPool,
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    provider: Arc<dyn LlmProvider>,
) -> Result<usize, AppError> {
    let Some(conversation) = conversations::get_conversation(conv_pool, conversation_id).await?
    else {
        tracing::warn!(conversation_id, "memory extraction conversation not found");
        return Ok(0);
    };
    let messages = conversations::list_messages(conv_pool, conversation_id).await?;
    let complete_user_count = messages
        .iter()
        .filter(|m| m.role == "user" && m.is_complete && !m.content.trim().is_empty())
        .count();
    let is_global_conversation = conversation.role_id.is_none();
    let has_delegations = messages
        .iter()
        .filter_map(|message| message.routing_metadata.as_deref())
        .any(has_delegations);

    if !is_global_conversation && complete_user_count < MIN_COMPLETE_USER_MESSAGES {
        return Ok(0);
    }
    if is_global_conversation
        && complete_user_count < MIN_COMPLETE_USER_MESSAGES
        && !has_delegations
    {
        return Ok(0);
    }

    let extracted = if is_global_conversation && complete_user_count < MIN_COMPLETE_USER_MESSAGES {
        Vec::new()
    } else {
        let global_messages = if is_global_conversation {
            user_extraction_messages(&global_extraction_messages(&messages))
        } else {
            user_extraction_messages(&messages)
        };
        let prompt = build_extraction_prompt(conversation.role_id.as_deref(), &global_messages);
        match extract_with_provider(provider.clone(), &prompt, &global_messages, conversation_id)
            .await
        {
            Ok(extracted) => extracted,
            Err(err) if is_global_conversation => {
                tracing::warn!(conversation_id, error = %err, "global memory extraction failed");
                Vec::new()
            }
            Err(err) => return Err(err),
        }
    };
    let mut total_inserted = 0;
    if extracted.is_empty() {
        tracing::debug!(
            conversation_id,
            "memory extraction produced no durable memories"
        );
    } else if conversation.role_id.is_some() {
        total_inserted += insert_reconciled_memories(
            main_pool,
            provider.clone(),
            conversation.role_id.as_deref(),
            conversation_id,
            &extracted,
        )
        .await?;
    } else {
        total_inserted += insert_global_memories_with_owners(
            main_pool,
            provider.clone(),
            conversation_id,
            &messages,
            &extracted,
        )
        .await?;
    }

    if conversation.role_id.is_none() {
        total_inserted +=
            extract_delegated_role_conversations(main_pool, conv_pool, provider, &messages).await?;
    }

    tracing::info!(
        conversation_id,
        role_id = ?conversation.role_id,
        inserted_count = total_inserted,
        "memory extraction completed"
    );
    Ok(total_inserted)
}

struct DelegatedExtractionTarget {
    role_id: String,
    conversation_id: Option<String>,
}

async fn extract_delegated_role_conversations(
    main_pool: &DbPool,
    conv_pool: &ConversationsPool,
    provider: Arc<dyn LlmProvider>,
    messages: &[Message],
) -> Result<usize, AppError> {
    let mut targets = latest_delegated_extraction_targets(messages);
    targets.sort_by(|a, b| {
        a.role_id
            .cmp(&b.role_id)
            .then_with(|| a.conversation_id.cmp(&b.conversation_id))
    });
    targets.dedup_by(|a, b| a.role_id == b.role_id && a.conversation_id == b.conversation_id);

    let mut inserted = 0;
    for target in targets {
        let has_recorded_conversation_id = target.conversation_id.is_some();
        let role_conversation_id = if let Some(conversation_id) = target.conversation_id {
            let conversation = match conversations::get_conversation(conv_pool, &conversation_id)
                .await
            {
                Ok(Some(conversation)) => conversation,
                Ok(None) => {
                    tracing::warn!(
                        role_id = target.role_id,
                        conversation_id,
                        "recorded delegated role conversation not found"
                    );
                    continue;
                }
                Err(err) => {
                    tracing::warn!(role_id = target.role_id, conversation_id, error = %err, "recorded delegated role conversation lookup failed");
                    continue;
                }
            };
            if conversation.role_id.as_deref() != Some(target.role_id.as_str()) {
                tracing::warn!(role_id = target.role_id, conversation_id, actual_role_id = ?conversation.role_id, "recorded delegated role conversation role mismatch");
                continue;
            }
            conversation.id
        } else {
            match conversations::get_or_create_conversation_by_role(conv_pool, &target.role_id)
                .await
            {
                Ok(conversation) => conversation.id,
                Err(err) => {
                    tracing::warn!(role_id = target.role_id, error = %err, "delegated role conversation lookup failed");
                    continue;
                }
            }
        };
        let min_complete_user_messages = if has_recorded_conversation_id {
            1
        } else {
            MIN_COMPLETE_USER_MESSAGES
        };
        match extract_role_conversation_with_provider(
            main_pool,
            conv_pool,
            &role_conversation_id,
            provider.clone(),
            min_complete_user_messages,
        )
        .await
        {
            Ok(count) => inserted += count,
            Err(err) => {
                tracing::warn!(role_id = target.role_id, error = %err, "delegated role memory extraction failed")
            }
        }
    }
    Ok(inserted)
}

async fn extract_role_conversation_with_provider(
    main_pool: &DbPool,
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    provider: Arc<dyn LlmProvider>,
    min_complete_user_messages: usize,
) -> Result<usize, AppError> {
    let Some(conversation) = conversations::get_conversation(conv_pool, conversation_id).await?
    else {
        return Ok(0);
    };
    let Some(role_id) = conversation.role_id.as_deref() else {
        return Ok(0);
    };
    let messages = conversations::list_messages(conv_pool, conversation_id).await?;
    let complete_user_count = messages
        .iter()
        .filter(|m| m.role == "user" && m.is_complete && !m.content.trim().is_empty())
        .count();
    if complete_user_count < min_complete_user_messages {
        return Ok(0);
    }

    let user_messages = user_extraction_messages(&messages);
    let prompt = build_extraction_prompt(Some(role_id), &user_messages);
    let extracted =
        extract_with_provider(provider.clone(), &prompt, &user_messages, conversation_id).await?;
    if extracted.is_empty() {
        return Ok(0);
    }
    insert_reconciled_memories(
        main_pool,
        provider,
        Some(role_id),
        conversation_id,
        &extracted,
    )
    .await
}

fn global_memory_candidates(
    messages: &[Message],
    extracted: &[ExtractedMemory],
) -> Vec<ExtractedMemory> {
    let delegated_message_ids = delegated_source_message_ids(messages);
    extracted
        .iter()
        .filter(|memory| {
            !memory
                .source_message_ids
                .iter()
                .any(|id| delegated_message_ids.contains(id))
        })
        .cloned()
        .collect()
}

async fn insert_reconciled_memories(
    main_pool: &DbPool,
    provider: Arc<dyn LlmProvider>,
    role_id: Option<&str>,
    source_conversation_id: &str,
    extracted: &[ExtractedMemory],
) -> Result<usize, AppError> {
    if extracted.is_empty() {
        return Ok(0);
    }

    let existing = memories::list_memories(main_pool, role_id, None, None, None).await?;
    let actions = match reconcile_memories(provider, role_id, &existing, extracted).await {
        Ok(actions) => actions,
        Err(err) => {
            tracing::warn!(role_id = ?role_id, source_conversation_id, error = %err, "memory reconciliation skipped");
            deterministic_reconciliation_actions(&existing, extracted)
        }
    };
    apply_memory_reconciliation(
        main_pool,
        role_id,
        source_conversation_id,
        extracted,
        actions,
    )
    .await
}

async fn insert_global_memories_with_owners(
    main_pool: &DbPool,
    provider: Arc<dyn LlmProvider>,
    conversation_id: &str,
    messages: &[Message],
    extracted: &[ExtractedMemory],
) -> Result<usize, AppError> {
    let candidates = global_memory_candidates(messages, extracted);
    if candidates.is_empty() {
        return Ok(0);
    }

    let roles = crate::db::roles::list_active_roles(main_pool).await?;
    let assignments = if roles.is_empty() {
        Vec::new()
    } else {
        match assign_memories_to_roles(provider.clone(), &roles, messages, &candidates).await {
            Ok(assignments) => assignments,
            Err(err) => {
                tracing::warn!(conversation_id, error = %err, "role memory owner assignment skipped");
                Vec::new()
            }
        }
    };
    let owner_by_index = unique_owner_by_memory_index(assignments, candidates.len());
    let mut grouped: BTreeMap<Option<String>, Vec<ExtractedMemory>> = BTreeMap::new();
    for (index, memory) in candidates.into_iter().enumerate() {
        let owner = if memory.category == "task_status" {
            None
        } else {
            owner_by_index.get(&index).cloned().flatten()
        };
        grouped.entry(owner).or_default().push(memory);
    }

    let mut changed = 0;
    for (owner, memories_for_owner) in grouped {
        changed += insert_reconciled_memories(
            main_pool,
            provider.clone(),
            owner.as_deref(),
            conversation_id,
            &memories_for_owner,
        )
        .await?;
    }
    Ok(changed)
}

fn unique_owner_by_memory_index(
    assignments: Vec<RoleMemoryAssignment>,
    memory_count: usize,
) -> BTreeMap<usize, Option<String>> {
    let mut owner_by_index = BTreeMap::new();
    let mut conflicts = HashSet::new();
    for assignment in assignments {
        for index in assignment.memory_indexes {
            if index >= memory_count {
                continue;
            }
            if owner_by_index
                .insert(index, Some(assignment.role_id.clone()))
                .is_some()
            {
                conflicts.insert(index);
            }
        }
    }
    for index in conflicts {
        owner_by_index.remove(&index);
    }
    owner_by_index
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MemoryReconciliationAction {
    Insert {
        memory_index: usize,
    },
    Skip {
        memory_index: usize,
    },
    Update {
        memory_index: usize,
        existing_memory_id: String,
    },
}

async fn reconcile_memories(
    provider: Arc<dyn LlmProvider>,
    role_id: Option<&str>,
    existing: &[Memory],
    candidates: &[ExtractedMemory],
) -> Result<Vec<MemoryReconciliationAction>, AppError> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    if existing.is_empty() {
        return Ok((0..candidates.len())
            .map(|memory_index| MemoryReconciliationAction::Insert { memory_index })
            .collect());
    }

    let deterministic = deterministic_reconciliation_actions(existing, candidates);
    let pending = deterministic
        .iter()
        .filter_map(|action| match action {
            MemoryReconciliationAction::Insert { memory_index } => Some(*memory_index),
            MemoryReconciliationAction::Skip { .. } | MemoryReconciliationAction::Update { .. } => {
                None
            }
        })
        .collect::<Vec<_>>();
    if pending.is_empty() {
        return Ok(deterministic);
    }

    let prompt = build_memory_reconciliation_prompt(role_id, existing, candidates, &pending);
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(128);
    let stream_handle = tokio::spawn(async move {
        if let Err(err) = provider
            .chat_stream(
                prompt,
                tx,
                ChatOptions {
                    disable_thinking: true,
                    tools: None,
                    tool_choice: None,
                },
            )
            .await
        {
            tracing::warn!(error = %err, "memory reconciliation provider stream failed");
        }
    });

    let mut response = String::new();
    let stream_result = timeout(Duration::from_secs(LLM_EXTRACTION_TIMEOUT_SECONDS), async {
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Token(token) => {
                    if response.len() + token.len() > MAX_EXTRACTION_RESPONSE_BYTES {
                        tracing::warn!("memory reconciliation response exceeded size limit");
                        return Ok(());
                    }
                    response.push_str(&token);
                }
                StreamEvent::Done => return Ok::<(), AppError>(()),
                StreamEvent::Error(err) => {
                    tracing::warn!(error = %err, "memory reconciliation stream error");
                    return Ok(());
                }
                StreamEvent::Thinking(_) | StreamEvent::ToolCall(_) => {}
            }
        }
        Ok(())
    })
    .await;
    if stream_result.is_err() {
        stream_handle.abort();
        tracing::warn!(role_id = ?role_id, "memory reconciliation timed out");
        return Ok(deterministic);
    }

    let llm_actions =
        parse_memory_reconciliation_response(&response, existing, candidates, &pending)?;
    Ok(merge_reconciliation_actions(deterministic, llm_actions))
}

fn deterministic_reconciliation_actions(
    existing: &[Memory],
    candidates: &[ExtractedMemory],
) -> Vec<MemoryReconciliationAction> {
    candidates
        .iter()
        .enumerate()
        .map(|(memory_index, candidate)| {
            let candidate_content = normalize_memory_content(&candidate.content);
            if existing.iter().any(|memory| {
                memory.category == candidate.category
                    && normalize_memory_content(&memory.content) == candidate_content
            }) {
                MemoryReconciliationAction::Skip { memory_index }
            } else {
                MemoryReconciliationAction::Insert { memory_index }
            }
        })
        .collect()
}

fn merge_reconciliation_actions(
    deterministic: Vec<MemoryReconciliationAction>,
    llm_actions: Vec<MemoryReconciliationAction>,
) -> Vec<MemoryReconciliationAction> {
    let llm_by_index = llm_actions
        .into_iter()
        .filter_map(|action| match &action {
            MemoryReconciliationAction::Insert { memory_index }
            | MemoryReconciliationAction::Skip { memory_index }
            | MemoryReconciliationAction::Update { memory_index, .. } => {
                Some((*memory_index, action))
            }
        })
        .collect::<BTreeMap<_, _>>();

    deterministic
        .into_iter()
        .map(|action| match action {
            MemoryReconciliationAction::Insert { memory_index } => llm_by_index
                .get(&memory_index)
                .cloned()
                .unwrap_or(MemoryReconciliationAction::Insert { memory_index }),
            action => action,
        })
        .collect()
}

async fn apply_memory_reconciliation(
    main_pool: &DbPool,
    role_id: Option<&str>,
    source_conversation_id: &str,
    extracted: &[ExtractedMemory],
    actions: Vec<MemoryReconciliationAction>,
) -> Result<usize, AppError> {
    let mut changed = 0;
    for action in actions {
        match action {
            MemoryReconciliationAction::Insert { memory_index } => {
                if let Some(memory) = extracted.get(memory_index) {
                    changed += memories::insert_memories(
                        main_pool,
                        role_id,
                        source_conversation_id,
                        std::slice::from_ref(memory),
                    )
                    .await?;
                }
            }
            MemoryReconciliationAction::Update {
                memory_index,
                existing_memory_id,
            } => {
                if let Some(memory) = extracted.get(memory_index) {
                    if memories::update_memory_from_extracted(
                        main_pool,
                        &existing_memory_id,
                        role_id,
                        source_conversation_id,
                        memory,
                    )
                    .await?
                    {
                        changed += 1;
                    }
                }
            }
            MemoryReconciliationAction::Skip { .. } => {}
        }
    }
    Ok(changed)
}

fn build_memory_reconciliation_prompt(
    role_id: Option<&str>,
    existing: &[Memory],
    candidates: &[ExtractedMemory],
    pending_indexes: &[usize],
) -> Vec<ChatCompletionMessage> {
    let existing_lines = existing
        .iter()
        .map(|memory| {
            format!(
                "id: {}\ncategory: {}\ncontent: {}\nsourceMessageIds: {}",
                memory.id, memory.category, memory.content, memory.source_message_ids
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n");
    let candidate_lines = pending_indexes
        .iter()
        .filter_map(|index| candidates.get(*index).map(|memory| (*index, memory)))
        .map(|(index, memory)| {
            format!(
                "index: {}\ncategory: {}\ncontent: {}\nsourceMessageIds: {}",
                index,
                memory.category,
                memory.content,
                serde_json::to_string(&memory.source_message_ids)
                    .unwrap_or_else(|_| "[]".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n");

    vec![
        ChatCompletionMessage {
            role: "system".to_string(),
            content: "你是 EgoSync 的记忆冲突审查器。只输出严格 JSON 对象，不要 Markdown、code fence 或解释文本。顶层格式必须是 {\"actions\":[{\"memoryIndex\":0,\"action\":\"insert|skip|update\",\"existingMemoryId\":\"...\"}]}。只审视同一个 owner 下的记忆；相同记忆输出 skip，新增记忆与旧记忆冲突且新记忆更新时输出 update，否则输出 insert。skip/update 必须引用 existingMemoryId。".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatCompletionMessage {
            role: "user".to_string(),
            content: format!(
                "owner: {}\n\nexisting memories:\n{}\n\ncandidate memories:\n{}",
                role_id.unwrap_or("butler"),
                existing_lines,
                candidate_lines
            ),
            tool_calls: None,
            tool_call_id: None,
        },
    ]
}

#[derive(Deserialize)]
struct ReconciliationResponse {
    actions: Vec<RawReconciliationAction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawReconciliationAction {
    memory_index: usize,
    action: String,
    existing_memory_id: Option<String>,
}

fn parse_memory_reconciliation_response(
    response: &str,
    existing: &[Memory],
    candidates: &[ExtractedMemory],
    allowed_indexes: &[usize],
) -> Result<Vec<MemoryReconciliationAction>, AppError> {
    let cleaned = strip_json_code_fence(response.trim());
    let parsed: ReconciliationResponse = serde_json::from_str(cleaned)
        .map_err(|e| AppError::ValidationError(format!("记忆冲突审查 JSON 解析失败: {}", e)))?;
    let existing_ids = existing
        .iter()
        .map(|memory| memory.id.as_str())
        .collect::<HashSet<_>>();
    let allowed = allowed_indexes.iter().copied().collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let actions = parsed
        .actions
        .into_iter()
        .filter_map(|action| {
            if action.memory_index >= candidates.len()
                || !allowed.contains(&action.memory_index)
                || !seen.insert(action.memory_index)
            {
                return None;
            }
            match action.action.trim() {
                "insert" => Some(MemoryReconciliationAction::Insert {
                    memory_index: action.memory_index,
                }),
                "skip" => action
                    .existing_memory_id
                    .as_deref()
                    .filter(|id| existing_ids.contains(*id))
                    .map(|_| MemoryReconciliationAction::Skip {
                        memory_index: action.memory_index,
                    }),
                "update" => action
                    .existing_memory_id
                    .filter(|id| existing_ids.contains(id.as_str()))
                    .map(|existing_memory_id| MemoryReconciliationAction::Update {
                        memory_index: action.memory_index,
                        existing_memory_id,
                    }),
                _ => None,
            }
        })
        .collect();
    Ok(actions)
}

fn normalize_memory_content(content: &str) -> String {
    let trimmed = content
        .trim()
        .trim_end_matches(|ch| matches!(ch, '.' | '。' | '!' | '！' | '?' | '？'));
    trimmed
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

struct RoleMemoryAssignment {
    role_id: String,
    memory_indexes: Vec<usize>,
}

async fn assign_memories_to_roles(
    provider: Arc<dyn LlmProvider>,
    roles: &[crate::models::role::Role],
    messages: &[Message],
    memories: &[ExtractedMemory],
) -> Result<Vec<RoleMemoryAssignment>, AppError> {
    let prompt = build_role_memory_assignment_prompt(roles, messages, memories);
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(128);
    let stream_handle = tokio::spawn(async move {
        if let Err(err) = provider
            .chat_stream(
                prompt,
                tx,
                ChatOptions {
                    disable_thinking: true,
                    tools: None,
                    tool_choice: None,
                },
            )
            .await
        {
            tracing::warn!(error = %err, "role memory assignment provider stream failed");
        }
    });

    let mut response = String::new();
    let stream_result = timeout(Duration::from_secs(LLM_EXTRACTION_TIMEOUT_SECONDS), async {
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Token(token) => {
                    if response.len() + token.len() > MAX_EXTRACTION_RESPONSE_BYTES {
                        tracing::warn!("role memory assignment response exceeded size limit");
                        return Ok(());
                    }
                    response.push_str(&token);
                }
                StreamEvent::Done => return Ok::<(), AppError>(()),
                StreamEvent::Error(err) => {
                    tracing::warn!(error = %err, "role memory assignment stream error");
                    return Ok(());
                }
                StreamEvent::Thinking(_) | StreamEvent::ToolCall(_) => {}
            }
        }
        Ok(())
    })
    .await;
    if stream_result.is_err() {
        stream_handle.abort();
        tracing::warn!("role memory assignment timed out");
        return Ok(Vec::new());
    }

    parse_role_memory_assignment_response(&response, roles, memories)
}

fn build_role_memory_assignment_prompt(
    roles: &[crate::models::role::Role],
    messages: &[Message],
    memories: &[ExtractedMemory],
) -> Vec<ChatCompletionMessage> {
    let role_lines = roles
        .iter()
        .map(|role| {
            format!(
                "id: {}\nname: {}\ngoal: {}",
                role.id,
                role.name,
                role.goal.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n");
    let memory_lines = memories
        .iter()
        .enumerate()
        .map(|(index, memory)| {
            format!(
                "index: {}\ncategory: {}\ncontent: {}\nsourceMessageIds: {}",
                index,
                memory.category,
                memory.content,
                serde_json::to_string(&memory.source_message_ids)
                    .unwrap_or_else(|_| "[]".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n");
    let transcript = messages
        .iter()
        .filter(|m| m.role != "system" && !m.content.trim().is_empty())
        .map(|m| {
            format!(
                "id: {}\nrole: {}\ncontent: {}",
                m.id,
                m.role,
                m.content.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n");

    vec![
        ChatCompletionMessage {
            role: "system".to_string(),
            content: "你是 EgoSync 的角色记忆归属判定器。只输出严格 JSON 对象，不要 Markdown、code fence 或解释文本。顶层格式必须是 {\"assignments\":[{\"memoryIndex\":0,\"roleId\":\"...|null\"}]}。每条 memory 最多归属一个 active 角色；只把关于某个 active 角色领域的事实、偏好、认知更新归属给该角色；任务状态或无法明确归属时 roleId 输出 null。".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatCompletionMessage {
            role: "user".to_string(),
            content: format!(
                "active roles:\n{}\n\nextracted memories:\n{}\n\nsource transcript:\n{}",
                role_lines, memory_lines, transcript
            ),
            tool_calls: None,
            tool_call_id: None,
        },
    ]
}

#[derive(Deserialize)]
struct RoleAssignmentResponse {
    assignments: Vec<RawRoleAssignment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRoleAssignment {
    role_id: Option<String>,
    #[serde(default)]
    memory_indexes: Vec<usize>,
    memory_index: Option<usize>,
}

fn parse_role_memory_assignment_response(
    response: &str,
    roles: &[crate::models::role::Role],
    memories: &[ExtractedMemory],
) -> Result<Vec<RoleMemoryAssignment>, AppError> {
    let cleaned = strip_json_code_fence(response.trim());
    let parsed: RoleAssignmentResponse = serde_json::from_str(cleaned)
        .map_err(|e| AppError::ValidationError(format!("角色记忆归属 JSON 解析失败: {}", e)))?;
    let role_ids = roles
        .iter()
        .map(|role| role.id.as_str())
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let assignments = parsed
        .assignments
        .into_iter()
        .filter_map(|assignment| {
            let role_id = assignment.role_id?.trim().to_string();
            if !role_ids.contains(role_id.as_str()) {
                return None;
            }
            let raw_indexes = if let Some(memory_index) = assignment.memory_index {
                vec![memory_index]
            } else {
                assignment.memory_indexes
            };
            let memory_indexes = raw_indexes
                .into_iter()
                .filter(|index| *index < memories.len())
                .filter(|index| seen.insert((role_id.clone(), *index)))
                .collect::<Vec<_>>();
            if memory_indexes.is_empty() {
                return None;
            }
            Some(RoleMemoryAssignment {
                role_id,
                memory_indexes,
            })
        })
        .collect();
    Ok(assignments)
}

fn delegated_source_message_ids(messages: &[Message]) -> HashSet<String> {
    messages
        .iter()
        .filter_map(|message| {
            let metadata = message.routing_metadata.as_deref()?;
            if has_delegations(metadata) {
                Some(message.id.clone())
            } else {
                None
            }
        })
        .collect()
}

fn latest_delegated_extraction_targets(messages: &[Message]) -> Vec<DelegatedExtractionTarget> {
    messages
        .iter()
        .rev()
        .find_map(|message| {
            let metadata = message.routing_metadata.as_deref()?;
            if has_delegations(metadata) {
                Some(successful_delegation_targets(metadata))
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn has_delegations(metadata: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(metadata)
        .ok()
        .and_then(|parsed| {
            parsed
                .get("delegations")
                .and_then(|d| d.as_array())
                .cloned()
        })
        .map(|delegations| !delegations.is_empty())
        .unwrap_or(false)
}

fn successful_delegation_targets(metadata: &str) -> Vec<DelegatedExtractionTarget> {
    serde_json::from_str::<serde_json::Value>(metadata)
        .ok()
        .and_then(|parsed| {
            parsed
                .get("delegations")
                .and_then(|d| d.as_array())
                .cloned()
        })
        .unwrap_or_default()
        .into_iter()
        .filter(|delegation| delegation.get("status").and_then(|v| v.as_str()) == Some("ok"))
        .filter_map(|delegation| {
            let role_id = delegation
                .get("targetRoleId")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|role_id| !role_id.is_empty())?
                .to_string();
            let conversation_id = delegation
                .get("targetConversationId")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|conversation_id| !conversation_id.is_empty())
                .map(str::to_string);
            Some(DelegatedExtractionTarget {
                role_id,
                conversation_id,
            })
        })
        .collect()
}

fn global_extraction_messages(messages: &[Message]) -> Vec<Message> {
    let delegated_message_ids = delegated_source_message_ids(messages);
    let mut excluded = HashSet::new();
    let mut exclude_until_next_user = false;

    for message in messages {
        if delegated_message_ids.contains(&message.id) {
            excluded.insert(message.id.clone());
            exclude_until_next_user = true;
            continue;
        }
        if exclude_until_next_user {
            if message.role == "user" {
                exclude_until_next_user = false;
            } else {
                excluded.insert(message.id.clone());
            }
        }
    }

    messages
        .iter()
        .filter(|message| !excluded.contains(&message.id))
        .cloned()
        .collect()
}

fn user_extraction_messages(messages: &[Message]) -> Vec<Message> {
    messages
        .iter()
        .filter(|message| {
            message.role == "user" && message.is_complete && !message.content.trim().is_empty()
        })
        .cloned()
        .collect()
}

async fn extract_with_provider(
    provider: Arc<dyn LlmProvider>,
    prompt: &[ChatCompletionMessage],
    messages: &[Message],
    conversation_id: &str,
) -> Result<Vec<ExtractedMemory>, AppError> {
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(128);
    let prompt = prompt.to_vec();
    let stream_handle = tokio::spawn(async move {
        if let Err(err) = provider
            .chat_stream(
                prompt,
                tx,
                ChatOptions {
                    disable_thinking: true,
                    tools: None,
                    tool_choice: None,
                },
            )
            .await
        {
            tracing::warn!(error = %err, "memory provider stream failed");
        }
    });

    let mut response = String::new();
    let mut response_too_large = false;
    let mut stream_failed = false;
    let stream_result = timeout(Duration::from_secs(LLM_EXTRACTION_TIMEOUT_SECONDS), async {
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Token(token) => {
                    if response.len() + token.len() > MAX_EXTRACTION_RESPONSE_BYTES {
                        response_too_large = true;
                        tracing::warn!(conversation_id, "memory LLM response exceeded size limit");
                        return Ok(());
                    }
                    response.push_str(&token);
                }
                StreamEvent::Done => return Ok::<(), AppError>(()),
                StreamEvent::Error(err) => {
                    stream_failed = true;
                    tracing::warn!(conversation_id, error = %err, "memory LLM stream error");
                    return Ok(());
                }
                StreamEvent::Thinking(_) | StreamEvent::ToolCall(_) => {}
            }
        }
        Ok(())
    })
    .await;
    if stream_result.is_err() {
        stream_handle.abort();
        tracing::warn!(conversation_id, "memory LLM stream timed out");
        return Ok(Vec::new());
    }
    if response_too_large {
        stream_handle.abort();
        return Ok(Vec::new());
    }
    if stream_failed {
        stream_handle.abort();
        return Ok(Vec::new());
    }

    parse_extraction_response(&response, messages)
}

fn build_extraction_prompt(
    role_id: Option<&str>,
    messages: &[Message],
) -> Vec<ChatCompletionMessage> {
    let scope = if role_id.is_some() {
        "角色专属记忆：只记录该角色领域内对未来有持续价值的偏好、任务状态、认知更新和事实。"
    } else {
        "管家全局记忆：只记录跨角色/全局可用的用户总体偏好、长期事实、价值观和沟通方式。"
    };
    let transcript = messages
        .iter()
        .filter(|m| m.role == "user" && !m.content.trim().is_empty())
        .map(|m| {
            format!(
                "id: {}\ncreated_at: {}\ncontent: {}",
                m.id,
                m.created_at,
                m.content.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n");

    vec![
        ChatCompletionMessage {
            role: "system".to_string(),
            content: format!(
                "你是 EgoSync 的记忆提炼器。{}\n只输出严格 JSON 对象，不要 Markdown、code fence 或解释文本。顶层格式必须是 {{\"memories\":[{{\"category\":\"preference|task_status|cognition_update|fact\",\"content\":\"...\",\"evidenceType\":\"explicit_statement|inferred_from_request\",\"evidenceText\":\"原始用户消息中的连续原文\",\"sourceMessageIds\":[\"...\"]}}]}}。无持久价值信息时输出 {{\"memories\":[]}}。只记录用户明确说出的长期信息；命令、任务要求和一次性操作不得推断为偏好、习惯、事实或状态。处理、阅读、总结材料不等于正在学习；一次指定 PDF/Word/Markdown 不等于格式偏好；一次搜索、创建或调用 Skill 不等于使用习惯。task_status 只能复述用户明确说出的正在进行、计划或承诺，不能从“请帮我做 X”推导“我正在做 X”。evidenceText 必须逐字摘自 sourceMessageIds 对应消息；直接陈述标记 explicit_statement，从请求推断的内容标记 inferred_from_request，后者不会被保存。不确定时输出空数组，宁可漏记，不可猜测。不要基于助手回复、角色回复或模型建议生成记忆。记忆内容用第一人称语境的自然事实表述，不要把对话对象称为“用户”，例如输出“儿子喜欢吃薯条”，不要输出“用户儿子喜欢吃薯条”。",
                scope
            ),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatCompletionMessage {
            role: "user".to_string(),
            content: format!("请从以下对话消息中提炼结构化记忆：\n\n{}", transcript),
            tool_calls: None,
            tool_call_id: None,
        },
    ]
}

#[derive(Deserialize)]
struct ExtractionResponse {
    memories: Vec<RawExtractedMemory>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawExtractedMemory {
    category: String,
    content: String,
    #[serde(default)]
    evidence_type: String,
    #[serde(default)]
    evidence_text: String,
    source_message_ids: Vec<String>,
}

fn contains_any(value: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| value.contains(marker))
}

fn has_supported_explicit_evidence(memory: &RawExtractedMemory, messages: &[Message]) -> bool {
    if memory.evidence_type != "explicit_statement" {
        return false;
    }
    let evidence = memory.evidence_text.trim();
    if evidence.is_empty()
        || !memory.source_message_ids.iter().any(|id| {
            messages.iter().any(|message| {
                message.role == "user" && message.id == *id && message.content.contains(evidence)
            })
        })
    {
        return false;
    }

    let durable_markers = [
        "我喜欢",
        "我偏好",
        "我更喜欢",
        "我的习惯",
        "我经常",
        "我通常",
        "以后",
        "今后",
        "默认",
        "请记住",
        "我正在",
        "我计划",
        "我准备",
        "我打算",
    ];
    let request_markers = ["请", "帮我", "麻烦", "把", "给我", "能否", "可以帮"];
    if contains_any(evidence, &request_markers) && !contains_any(evidence, &durable_markers) {
        return false;
    }

    let claim = memory.content.as_str();
    let signal_groups: &[(&[&str], &[&str])] = &[
        (
            &["偏好", "喜欢", "默认"],
            &["偏好", "喜欢", "更喜欢", "默认", "以后", "今后"],
        ),
        (
            &["习惯", "经常", "通常", "总是", "一直"],
            &["习惯", "经常", "通常", "总是", "一直"],
        ),
        (
            &["正在", "计划", "准备", "打算"],
            &["正在", "计划", "准备", "打算"],
        ),
        (&["学习"], &["学习", "在学"]),
    ];
    signal_groups
        .iter()
        .all(|(claim_markers, evidence_markers)| {
            !contains_any(claim, claim_markers) || contains_any(evidence, evidence_markers)
        })
}

fn parse_extraction_response(
    response: &str,
    messages: &[Message],
) -> Result<Vec<ExtractedMemory>, AppError> {
    let cleaned = strip_json_code_fence(response.trim());
    let parsed: ExtractionResponse = serde_json::from_str(cleaned)
        .map_err(|e| AppError::ValidationError(format!("记忆提炼 JSON 解析失败: {}", e)))?;
    let memories = parsed
        .memories
        .into_iter()
        .filter_map(|memory| {
            let content = sanitize_memory_content(&memory.content);
            if !ALLOWED_CATEGORIES.contains(&memory.category.as_str()) {
                tracing::warn!(
                    category = memory.category,
                    "memory extraction dropped invalid category"
                );
                return None;
            }
            if content.is_empty() {
                return None;
            }
            if memory.source_message_ids.is_empty()
                || memory.source_message_ids.iter().any(|id| {
                    !messages
                        .iter()
                        .any(|message| message.role == "user" && message.id == *id)
                })
            {
                tracing::warn!("memory extraction dropped invalid source ids");
                return None;
            }
            if !has_supported_explicit_evidence(&memory, messages) {
                tracing::warn!("memory extraction dropped unsupported evidence");
                return None;
            }
            Some(ExtractedMemory {
                category: memory.category,
                content,
                source_message_ids: memory.source_message_ids,
            })
        })
        .collect();
    Ok(memories)
}

fn sanitize_memory_content(content: &str) -> String {
    let mut value = content.trim().to_string();
    for (from, to) in [
        ("用户的儿子", "儿子"),
        ("用户儿子", "儿子"),
        ("用户的女儿", "女儿"),
        ("用户女儿", "女儿"),
        ("用户的孩子", "孩子"),
        ("用户孩子", "孩子"),
        ("用户的父亲", "父亲"),
        ("用户父亲", "父亲"),
        ("用户的母亲", "母亲"),
        ("用户母亲", "母亲"),
        ("用户的家庭", "家庭"),
        ("用户家庭", "家庭"),
    ] {
        value = value.replace(from, to);
    }
    for prefix in [
        "用户比较",
        "用户偏好",
        "用户喜欢",
        "用户经常",
        "用户正在",
        "用户将",
        "用户会",
        "用户要",
        "用户",
    ] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            value = stripped
                .trim_start_matches(['的', '：', ':', '，', ','])
                .trim()
                .to_string();
            break;
        }
    }
    value
}

fn strip_json_code_fence(response: &str) -> &str {
    let Some(start) = response.find('{') else {
        return response;
    };
    let Some(end) = response.rfind('}') else {
        return response;
    };
    if start > end {
        return response;
    }
    response[start..=end].trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::Message;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_main_pool() -> DbPool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create main db");
        sqlx::raw_sql(include_str!("../../migrations/001_initial_schema.sql"))
            .execute(&pool)
            .await
            .expect("create settings schema");
        sqlx::raw_sql(include_str!("../../migrations/003_roles.sql"))
            .execute(&pool)
            .await
            .expect("create roles schema");
        sqlx::raw_sql(include_str!("../../migrations/019_energy_updated_at.sql"))
            .execute(&pool)
            .await
            .expect("add energy_updated_at column");
        sqlx::raw_sql(include_str!("../../migrations/004_memories.sql"))
            .execute(&pool)
            .await
            .expect("create memories schema");
        sqlx::raw_sql(include_str!(
            "../../migrations/005_memory_role_scoped_dedupe.sql"
        ))
        .execute(&pool)
        .await
        .expect("migrate memory dedupe index");
        sqlx::raw_sql(include_str!(
            "../../migrations/006_memory_single_owner_dedupe.sql"
        ))
        .execute(&pool)
        .await
        .expect("migrate memory single-owner dedupe index");
        sqlx::raw_sql(include_str!(
            "../../migrations/007_forgotten_memory_sources.sql"
        ))
        .execute(&pool)
        .await
        .expect("create forgotten memory sources");
        pool
    }

    async fn setup_conversation_pool() -> ConversationsPool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create conversations db");
        sqlx::raw_sql(include_str!("../../migrations/002_conversations.sql"))
            .execute(&pool)
            .await
            .expect("create conversations schema");
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN thinking_content TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("add thinking_content");
        sqlx::raw_sql("ALTER TABLE conversations ADD COLUMN title TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("add title");
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN routing_metadata TEXT")
            .execute(&pool)
            .await
            .expect("add routing_metadata");
        ConversationsPool(pool)
    }

    fn message(id: &str, role: &str, content: &str) -> Message {
        Message {
            id: id.to_string(),
            conversation_id: "conv-1".to_string(),
            role: role.to_string(),
            content: content.to_string(),
            thinking_content: String::new(),
            is_complete: true,
            created_at: "2026-05-30T00:00:00Z".to_string(),
            routing_metadata: None,
        }
    }

    #[test]
    fn parse_extraction_response_accepts_explicit_supported_evidence() {
        let messages = vec![
            message("msg-1", "user", "我偏好中文输出，并重视可验证结论"),
            message("msg-2", "user", "我正在做 Story 2.6"),
        ];
        let strict = parse_extraction_response(
            r#"{"memories":[{"category":"preference","content":"用户喜欢中文输出","evidenceType":"explicit_statement","evidenceText":"我偏好中文输出","sourceMessageIds":["msg-1"]}]}"#,
            &messages,
        )
        .expect("strict json parses");
        let fenced = parse_extraction_response(
            "```json\n{\"memories\":[{\"category\":\"fact\",\"content\":\"用户正在做 Story 2.6\",\"evidenceType\":\"explicit_statement\",\"evidenceText\":\"我正在做 Story 2.6\",\"sourceMessageIds\":[\"msg-2\"]}]}\n```",
            &messages,
        )
        .expect("fenced json parses");
        let surrounded = parse_extraction_response(
            "下面是结果：\n~~~Json\n{\"memories\":[{\"category\":\"fact\",\"content\":\"用户重视可验证结论\",\"evidenceType\":\"explicit_statement\",\"evidenceText\":\"重视可验证结论\",\"sourceMessageIds\":[\"msg-1\"]}]}\n~~~\n请查收。",
            &messages,
        )
        .expect("surrounded json parses");

        assert_eq!(strict.len(), 1);
        assert_eq!(strict[0].category, "preference");
        assert_eq!(strict[0].content, "中文输出");
        assert_eq!(strict[0].source_message_ids, vec!["msg-1"]);
        assert_eq!(fenced.len(), 1);
        assert_eq!(fenced[0].content, "做 Story 2.6");
        assert_eq!(surrounded.len(), 1);
        assert_eq!(surrounded[0].content, "重视可验证结论");
    }

    #[test]
    fn parse_extraction_response_rejects_one_off_requests_as_durable_attributes() {
        let messages = vec![
            message(
                "msg-1",
                "user",
                "帮我处理并总结这份北京大学 Harness Engineering 课程文档",
            ),
            message("msg-2", "user", "把截取内容保存为 PDF"),
            message("msg-3", "user", "帮我搜索并创建一个 Skill"),
        ];
        let parsed = parse_extraction_response(
            r#"{
                "memories": [
                    {"category":"task_status","content":"正在学习北京大学 Harness Engineering 课程","evidenceType":"explicit_statement","evidenceText":"帮我处理并总结这份北京大学 Harness Engineering 课程文档","sourceMessageIds":["msg-1"]},
                    {"category":"preference","content":"我偏好将文档截取内容保存为 PDF 格式","evidenceType":"inferred_from_request","evidenceText":"把截取内容保存为 PDF","sourceMessageIds":["msg-2"]},
                    {"category":"fact","content":"我有使用 Skill 平台扩展功能的习惯","evidenceType":"explicit_statement","evidenceText":"帮我搜索并创建一个 Skill","sourceMessageIds":["msg-3"]}
                ]
            }"#,
            &messages,
        )
        .expect("json parses");

        assert!(parsed.is_empty(), "一次性任务不能变成长期记忆");
    }

    #[test]
    fn parse_extraction_response_keeps_matching_explicit_durable_statements() {
        let messages = vec![
            message(
                "msg-1",
                "user",
                "我正在系统学习北京大学的 Harness Engineering 课程",
            ),
            message("msg-2", "user", "以后这类文档默认给我 PDF，我偏好 PDF"),
            message("msg-3", "user", "我经常使用 Skill 扩展功能，请记住这个习惯"),
        ];
        let parsed = parse_extraction_response(
            r#"{
                "memories": [
                    {"category":"task_status","content":"正在系统学习北京大学的 Harness Engineering 课程","evidenceType":"explicit_statement","evidenceText":"我正在系统学习北京大学的 Harness Engineering 课程","sourceMessageIds":["msg-1"]},
                    {"category":"preference","content":"偏好 PDF 格式","evidenceType":"explicit_statement","evidenceText":"以后这类文档默认给我 PDF，我偏好 PDF","sourceMessageIds":["msg-2"]},
                    {"category":"fact","content":"经常使用 Skill 扩展功能","evidenceType":"explicit_statement","evidenceText":"我经常使用 Skill 扩展功能，请记住这个习惯","sourceMessageIds":["msg-3"]}
                ]
            }"#,
            &messages,
        )
        .expect("json parses");

        assert_eq!(parsed.len(), 3, "明确陈述的长期信息仍应保留");
    }

    #[test]
    fn parse_extraction_response_filters_invalid_items_and_empty_output() {
        let messages = vec![message("msg-1", "user", "请记住：我正在推进记忆管线")];
        let parsed = parse_extraction_response(
            r#"{
                "memories": [
                    {"category":"invalid","content":"错误分类","evidenceType":"explicit_statement","evidenceText":"我正在推进记忆管线","sourceMessageIds":["msg-1"]},
                    {"category":"fact","content":"   ","evidenceType":"explicit_statement","evidenceText":"我正在推进记忆管线","sourceMessageIds":["msg-1"]},
                    {"category":"fact","content":"错误来源","evidenceType":"explicit_statement","evidenceText":"我正在推进记忆管线","sourceMessageIds":["other"]},
                    {"category":"task_status","content":"用户正在推进记忆管线","evidenceType":"explicit_statement","evidenceText":"我正在推进记忆管线","sourceMessageIds":["msg-1"]}
                ]
            }"#,
            &messages,
        )
        .expect("json parses");
        let empty =
            parse_extraction_response(r#"{"memories":[]}"#, &messages).expect("empty parses");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].category, "task_status");
        assert_eq!(parsed[0].content, "推进记忆管线");
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn parse_extraction_response_rejects_natural_language() {
        let messages = vec![message("msg-1", "user", "我偏好中文")];
        let err = parse_extraction_response("这里没有值得记录的记忆。", &messages)
            .expect_err("natural language is not valid extraction json");
        assert!(matches!(err, crate::error::AppError::ValidationError(_)));
    }

    #[test]
    fn build_extraction_prompt_excludes_system_and_empty_messages() {
        let messages = vec![
            message("sys", "system", "[onboarding_start]"),
            message("empty", "user", "   "),
            message("msg-1", "user", "我喜欢中文沟通"),
            message("msg-2", "assistant", "好的"),
        ];

        let prompt = build_extraction_prompt(None, &messages);

        assert!(prompt.iter().any(|m| m.content.contains("管家全局记忆")));
        assert!(prompt.iter().any(|m| m.content.contains("msg-1")));
        assert!(!prompt.iter().any(|m| m.content.contains("msg-2")));
        assert!(!prompt.iter().any(|m| m.content.contains("好的")));
        assert!(!prompt
            .iter()
            .any(|m| m.content.contains("[onboarding_start]")));
        assert!(!prompt.iter().any(|m| m.content.contains("empty")));
    }

    struct FakeMemoryProvider {
        response: String,
    }

    impl FakeMemoryProvider {
        fn new(
            category: &str,
            content: &str,
            source_message_id: &str,
            evidence_text: &str,
        ) -> Self {
            Self {
                response: serde_json::json!({
                    "memories": [{
                        "category": category,
                        "content": content,
                        "evidenceType": "explicit_statement",
                        "evidenceText": evidence_text,
                        "evidenceType":"explicit_statement","evidenceText":"请记住：我偏好中文沟通，并正在推进任务 0","sourceMessageIds": [source_message_id],
                    }]
                })
                .to_string(),
            }
        }

        fn from_response(response: String) -> Self {
            Self { response }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for FakeMemoryProvider {
        async fn test_connection(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn chat_stream(
            &self,
            _messages: Vec<ChatCompletionMessage>,
            tx: mpsc::Sender<StreamEvent>,
            _options: ChatOptions,
        ) -> Result<(), AppError> {
            tx.send(StreamEvent::Token(self.response.clone()))
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            tx.send(StreamEvent::Done)
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            Ok(())
        }
    }

    struct GlobalRoleAssignmentProvider {
        extraction_response: String,
        assignment_response: String,
    }

    impl GlobalRoleAssignmentProvider {
        fn new(extraction_response: String, assignment_response: String) -> Self {
            Self {
                extraction_response,
                assignment_response,
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for GlobalRoleAssignmentProvider {
        async fn test_connection(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn chat_stream(
            &self,
            messages: Vec<ChatCompletionMessage>,
            tx: mpsc::Sender<StreamEvent>,
            _options: ChatOptions,
        ) -> Result<(), AppError> {
            let prompt = messages
                .iter()
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let response = if prompt.contains("角色记忆归属判定器")
                || prompt.contains("active roles:")
            {
                self.assignment_response.clone()
            } else {
                self.extraction_response.clone()
            };
            tx.send(StreamEvent::Token(response))
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            tx.send(StreamEvent::Done)
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            Ok(())
        }
    }

    struct ScopedMemoryProvider {
        role_source_message_id: String,
    }

    impl ScopedMemoryProvider {
        fn new(role_source_message_id: &str) -> Self {
            Self {
                role_source_message_id: role_source_message_id.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for ScopedMemoryProvider {
        async fn test_connection(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn chat_stream(
            &self,
            messages: Vec<ChatCompletionMessage>,
            tx: mpsc::Sender<StreamEvent>,
            _options: ChatOptions,
        ) -> Result<(), AppError> {
            let prompt = messages
                .iter()
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let response = if prompt.contains("角色专属记忆") {
                format!(
                    r#"{{"memories":[{{"category":"task_status","content":"产品经理正在准备设计评审","evidenceType":"explicit_statement","evidenceText":"请记住：我偏好中文沟通，并正在推进任务 0","sourceMessageIds":["{}"]}}]}}"#,
                    self.role_source_message_id
                )
            } else {
                r#"{"memories":[]}"#.to_string()
            };
            tx.send(StreamEvent::Token(response))
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            tx.send(StreamEvent::Done)
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            Ok(())
        }
    }

    struct DelegatedTurnLeakProvider {
        assistant_source_message_id: String,
    }

    impl DelegatedTurnLeakProvider {
        fn new(assistant_source_message_id: &str) -> Self {
            Self {
                assistant_source_message_id: assistant_source_message_id.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for DelegatedTurnLeakProvider {
        async fn test_connection(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn chat_stream(
            &self,
            messages: Vec<ChatCompletionMessage>,
            tx: mpsc::Sender<StreamEvent>,
            _options: ChatOptions,
        ) -> Result<(), AppError> {
            let prompt = messages
                .iter()
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let response = if prompt.contains("产品经理建议调整设计评审") {
                format!(
                    r#"{{"memories":[{{"category":"task_status","content":"产品经理正在调整设计评审","evidenceType":"explicit_statement","evidenceText":"请记住：我偏好中文沟通，并正在推进任务 0","sourceMessageIds":["{}"]}}]}}"#,
                    self.assistant_source_message_id
                )
            } else {
                r#"{"memories":[]}"#.to_string()
            };
            tx.send(StreamEvent::Token(response))
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            tx.send(StreamEvent::Done)
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            Ok(())
        }
    }

    struct RolePromptProvider {
        role_1_source_message_id: String,
        role_1_evidence_text: String,
        role_2_source_message_id: String,
        role_2_evidence_text: String,
    }

    impl RolePromptProvider {
        fn new(
            role_1_source_message_id: &str,
            role_1_evidence_text: &str,
            role_2_source_message_id: &str,
            role_2_evidence_text: &str,
        ) -> Self {
            Self {
                role_1_source_message_id: role_1_source_message_id.to_string(),
                role_1_evidence_text: role_1_evidence_text.to_string(),
                role_2_source_message_id: role_2_source_message_id.to_string(),
                role_2_evidence_text: role_2_evidence_text.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for RolePromptProvider {
        async fn test_connection(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn chat_stream(
            &self,
            messages: Vec<ChatCompletionMessage>,
            tx: mpsc::Sender<StreamEvent>,
            _options: ChatOptions,
        ) -> Result<(), AppError> {
            let prompt = messages
                .iter()
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let response = if prompt.contains(&self.role_1_source_message_id) {
                serde_json::json!({
                    "memories": [{
                        "category": "task_status",
                        "content": "角色一历史委派记忆",
                        "evidenceType": "explicit_statement",
                        "evidenceText": self.role_1_evidence_text,
                        "sourceMessageIds": [self.role_1_source_message_id],
                    }]
                })
                .to_string()
            } else if prompt.contains(&self.role_2_source_message_id) {
                serde_json::json!({
                    "memories": [{
                        "category": "task_status",
                        "content": "角色二最新委派记忆",
                        "evidenceType": "explicit_statement",
                        "evidenceText": self.role_2_evidence_text,
                        "sourceMessageIds": [self.role_2_source_message_id],
                    }]
                })
                .to_string()
            } else {
                r#"{"memories":[]}"#.to_string()
            };
            tx.send(StreamEvent::Token(response))
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            tx.send(StreamEvent::Done)
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            Ok(())
        }
    }

    struct GlobalErrorRolePromptProvider {
        role_source_message_id: String,
    }

    impl GlobalErrorRolePromptProvider {
        fn new(role_source_message_id: &str) -> Self {
            Self {
                role_source_message_id: role_source_message_id.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for GlobalErrorRolePromptProvider {
        async fn test_connection(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn chat_stream(
            &self,
            messages: Vec<ChatCompletionMessage>,
            tx: mpsc::Sender<StreamEvent>,
            _options: ChatOptions,
        ) -> Result<(), AppError> {
            let prompt = messages
                .iter()
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let response = if prompt.contains("角色专属记忆") {
                format!(
                    r#"{{"memories":[{{"category":"task_status","content":"角色记忆不应被全局错误阻断","evidenceType":"explicit_statement","evidenceText":"请记住：我偏好中文沟通，并正在推进任务 0","sourceMessageIds":["{}"]}}]}}"#,
                    self.role_source_message_id
                )
            } else {
                "全局提取返回了非 JSON 内容".to_string()
            };
            tx.send(StreamEvent::Token(response))
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            tx.send(StreamEvent::Done)
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            Ok(())
        }
    }

    struct RoleOneErrorRoleTwoOkProvider {
        role_1_source_message_id: String,
        role_2_source_message_id: String,
    }

    impl RoleOneErrorRoleTwoOkProvider {
        fn new(role_1_source_message_id: &str, role_2_source_message_id: &str) -> Self {
            Self {
                role_1_source_message_id: role_1_source_message_id.to_string(),
                role_2_source_message_id: role_2_source_message_id.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for RoleOneErrorRoleTwoOkProvider {
        async fn test_connection(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn chat_stream(
            &self,
            messages: Vec<ChatCompletionMessage>,
            tx: mpsc::Sender<StreamEvent>,
            _options: ChatOptions,
        ) -> Result<(), AppError> {
            let prompt = messages
                .iter()
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let response = if prompt.contains(&self.role_1_source_message_id) {
                "角色一提取返回非 JSON".to_string()
            } else if prompt.contains(&self.role_2_source_message_id) {
                format!(
                    r#"{{"memories":[{{"category":"task_status","content":"角色二不应被角色一错误阻断","evidenceType":"explicit_statement","evidenceText":"请记住：我偏好中文沟通，并正在推进任务 0","sourceMessageIds":["{}"]}}]}}"#,
                    self.role_2_source_message_id
                )
            } else {
                r#"{"memories":[]}"#.to_string()
            };
            tx.send(StreamEvent::Token(response))
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            tx.send(StreamEvent::Done)
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            Ok(())
        }
    }

    struct ErrorAfterTokenProvider {
        token: String,
    }

    #[async_trait::async_trait]
    impl LlmProvider for ErrorAfterTokenProvider {
        async fn test_connection(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn chat_stream(
            &self,
            _messages: Vec<ChatCompletionMessage>,
            tx: mpsc::Sender<StreamEvent>,
            _options: ChatOptions,
        ) -> Result<(), AppError> {
            tx.send(StreamEvent::Token(self.token.clone()))
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            tx.send(StreamEvent::Error("provider failed".to_string()))
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            Ok(())
        }
    }

    struct ReconciliationProvider {
        response: String,
    }

    impl ReconciliationProvider {
        fn new(response: &str) -> Self {
            Self {
                response: response.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for ReconciliationProvider {
        async fn test_connection(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn chat_stream(
            &self,
            _messages: Vec<ChatCompletionMessage>,
            tx: mpsc::Sender<StreamEvent>,
            _options: ChatOptions,
        ) -> Result<(), AppError> {
            tx.send(StreamEvent::Token(self.response.clone()))
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            tx.send(StreamEvent::Done)
                .await
                .map_err(|e| AppError::LlmError(format!("fake stream failed: {}", e)))?;
            Ok(())
        }
    }

    fn extracted(category: &str, content: &str, source_message_ids: Vec<&str>) -> ExtractedMemory {
        ExtractedMemory {
            category: category.to_string(),
            content: content.to_string(),
            source_message_ids: source_message_ids.into_iter().map(str::to_string).collect(),
        }
    }

    async fn insert_complete_user_messages(
        pool: &ConversationsPool,
        conversation_id: &str,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        for i in 0..3 {
            messages.push(
                conversations::insert_message(
                    pool,
                    conversation_id,
                    "user",
                    &format!("请记住：我偏好中文沟通，并正在推进任务 {}", i),
                    true,
                )
                .await
                .unwrap(),
            );
        }
        messages
    }

    #[tokio::test]
    async fn insert_reconciled_memories_skips_same_memory() {
        let main_pool = setup_main_pool().await;
        let first = extracted("fact", "用户喜欢中文输出。", vec!["msg-1"]);
        memories::insert_memories(&main_pool, None, "conv-old", &[first])
            .await
            .expect("insert existing memory");

        let changed = insert_reconciled_memories(
            &main_pool,
            Arc::new(ReconciliationProvider::new(r#"{"actions":[]}"#)),
            None,
            "conv-new",
            &[extracted("fact", "  用户喜欢中文输出  ", vec!["msg-2"])],
        )
        .await
        .expect("reconcile memories");

        let rows: Vec<String> = sqlx::query_scalar("SELECT content FROM memories")
            .fetch_all(&main_pool)
            .await
            .expect("query memories");
        assert_eq!(changed, 0);
        assert_eq!(rows, vec!["用户喜欢中文输出。"]);
    }

    #[tokio::test]
    async fn insert_reconciled_memories_skips_forgotten_same_source_candidate() {
        let main_pool = setup_main_pool().await;
        memories::insert_memories(
            &main_pool,
            None,
            "conv-old",
            &[extracted("preference", "儿子喜欢吃薯条", vec!["msg-1"])],
        )
        .await
        .expect("insert existing memory");
        let memory_id: String = sqlx::query_scalar("SELECT id FROM memories")
            .fetch_one(&main_pool)
            .await
            .expect("query memory id");
        memories::delete_memory(&main_pool, &memory_id)
            .await
            .expect("forget memory");

        let changed = insert_reconciled_memories(
            &main_pool,
            Arc::new(ReconciliationProvider::new(r#"{"actions":[]}"#)),
            None,
            "conv-old",
            &[extracted("preference", "儿子真的喜欢吃薯条", vec!["msg-1"])],
        )
        .await
        .expect("reconcile forgotten source");
        let visible_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(&main_pool)
            .await
            .expect("count memories");

        assert_eq!(changed, 0);
        assert_eq!(visible_count, 0);
    }

    #[tokio::test]
    async fn insert_reconciled_memories_updates_conflicting_memory() {
        let main_pool = setup_main_pool().await;
        memories::insert_memories(
            &main_pool,
            None,
            "conv-old",
            &[extracted("fact", "用户偏好每周一上午开会", vec!["msg-1"])],
        )
        .await
        .expect("insert existing memory");
        let existing_id: String = sqlx::query_scalar("SELECT id FROM memories")
            .fetch_one(&main_pool)
            .await
            .expect("query existing id");
        let response = format!(
            r#"{{"actions":[{{"memoryIndex":0,"action":"update","existingMemoryId":"{}"}}]}}"#,
            existing_id
        );

        let changed = insert_reconciled_memories(
            &main_pool,
            Arc::new(ReconciliationProvider::new(&response)),
            None,
            "conv-new",
            &[extracted("fact", "用户偏好每周二下午开会", vec!["msg-2"])],
        )
        .await
        .expect("reconcile memories");

        let stored: Vec<(String, String, String)> =
            sqlx::query_as("SELECT id, content, source_conversation_id FROM memories")
                .fetch_all(&main_pool)
                .await
                .expect("query memories");
        assert_eq!(changed, 1);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].0, existing_id);
        assert_eq!(stored[0].1, "用户偏好每周二下午开会");
        assert_eq!(stored[0].2, "conv-new");
    }

    #[tokio::test]
    async fn extract_for_conversation_routes_global_and_role_memories() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品经理')")
            .execute(&main_pool)
            .await
            .expect("insert role");
        let global_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let role_conv = conversations::create_conversation(&conv_pool, Some("role-1"))
            .await
            .unwrap();
        let global_messages = insert_complete_user_messages(&conv_pool, &global_conv.id).await;
        let role_messages = insert_complete_user_messages(&conv_pool, &role_conv.id).await;

        let global_count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &global_conv.id,
            Arc::new(FakeMemoryProvider::new(
                "fact",
                "用户偏好中文沟通",
                &global_messages[0].id,
                &global_messages[0].content,
            )),
        )
        .await
        .expect("global extraction succeeds");
        let role_count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &role_conv.id,
            Arc::new(FakeMemoryProvider::new(
                "preference",
                "产品规划需要表格输出",
                &role_messages[0].id,
                &role_messages[0].content,
            )),
        )
        .await
        .expect("role extraction succeeds");

        let global_role_id: Option<String> =
            sqlx::query_scalar("SELECT role_id FROM memories WHERE source_conversation_id = ?1")
                .bind(&global_conv.id)
                .fetch_one(&main_pool)
                .await
                .expect("query global memory");
        let role_id: Option<String> =
            sqlx::query_scalar("SELECT role_id FROM memories WHERE source_conversation_id = ?1")
                .bind(&role_conv.id)
                .fetch_one(&main_pool)
                .await
                .expect("query role memory");

        assert_eq!(global_count, 1);
        assert_eq!(role_count, 1);
        assert_eq!(global_role_id, None);
        assert_eq!(role_id.as_deref(), Some("role-1"));
    }

    #[tokio::test]
    async fn extract_for_conversation_assigns_single_owner_for_role_facts_from_butler_memory() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query(
            "INSERT INTO roles (id, name, icon, color, goal, status) VALUES ('father-role', '父亲', 'baby', '#EF4444', '照顾孩子成长', 'active')",
        )
        .execute(&main_pool)
        .await
        .expect("insert role");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let msg_1 = conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "我明天下午跟儿子去吃麦当劳，他比较喜欢吃薯条",
            true,
        )
        .await
        .unwrap();
        conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "他的阅读和英语还可以",
            true,
        )
        .await
        .unwrap();
        conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "但数学计算的能力不是太好",
            true,
        )
        .await
        .unwrap();
        let extraction_response = format!(
            r#"{{"memories":[{{"category":"preference","content":"儿子喜欢吃薯条","evidenceType":"explicit_statement","evidenceText":"他比较喜欢吃薯条","sourceMessageIds":["{}"]}}]}}"#,
            msg_1.id
        );

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(GlobalRoleAssignmentProvider::new(
                extraction_response,
                r#"{"assignments":[{"roleId":"father-role","memoryIndexes":[0]}]}"#.to_string(),
            )),
        )
        .await
        .expect("global extraction succeeds");

        let stored: Vec<(Option<String>, String)> = sqlx::query_as(
            "SELECT role_id, content FROM memories ORDER BY role_id IS NOT NULL, role_id",
        )
        .fetch_all(&main_pool)
        .await
        .expect("query memories");
        assert_eq!(count, 1);
        assert_eq!(
            stored,
            vec![(
                Some("father-role".to_string()),
                "儿子喜欢吃薯条".to_string()
            )]
        );
    }

    #[tokio::test]
    async fn extract_for_conversation_does_not_sync_task_status_as_role_fact() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query(
            "INSERT INTO roles (id, name, icon, color, goal, status) VALUES ('father-role', '父亲', 'baby', '#EF4444', '照顾孩子成长', 'active')",
        )
        .execute(&main_pool)
        .await
        .expect("insert role");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        insert_complete_user_messages(&conv_pool, &butler_conv.id).await;
        let status_message = conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "我明天下午要参加儿子的家长会",
            true,
        )
        .await
        .unwrap();
        let extraction_response = serde_json::json!({
            "memories": [{
                "category": "task_status",
                "content": "明天下午要参加儿子的家长会",
                "evidenceType": "explicit_statement",
                "evidenceText": status_message.content,
                "sourceMessageIds": [status_message.id],
            }]
        })
        .to_string();

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(GlobalRoleAssignmentProvider::new(
                extraction_response,
                r#"{"assignments":[{"roleId":"father-role","memoryIndexes":[0]}]}"#.to_string(),
            )),
        )
        .await
        .expect("global extraction succeeds");

        let stored: Vec<(Option<String>, String)> =
            sqlx::query_as("SELECT role_id, content FROM memories")
                .fetch_all(&main_pool)
                .await
                .expect("query memories");
        assert_eq!(count, 1);
        assert_eq!(
            stored,
            vec![(None, "明天下午要参加儿子的家长会".to_string())]
        );
    }

    #[tokio::test]
    async fn extract_for_conversation_routes_delegated_butler_memory_to_target_role() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品经理')")
            .execute(&main_pool)
            .await
            .expect("insert role");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let mut messages = insert_complete_user_messages(&conv_pool, &butler_conv.id).await;
        let delegated_user = messages.remove(0);
        conversations::update_message_routing_metadata(
            &conv_pool,
            &delegated_user.id,
            r#"{"delegations":[{"targetRoleId":"role-1","targetRoleName":"产品经理","taskSummary":"准备设计评审","status":"ok"}]}"#,
        )
        .await
        .expect("write routing metadata");

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(FakeMemoryProvider::new(
                "task_status",
                "产品经理正在准备设计评审",
                &delegated_user.id,
                &delegated_user.content,
            )),
        )
        .await
        .expect("delegated extraction succeeds");

        let stored_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(&main_pool)
            .await
            .expect("count memories");
        assert_eq!(count, 0);
        assert_eq!(stored_count, 0);
    }

    #[tokio::test]
    async fn extract_for_conversation_also_extracts_delegated_role_conversations() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品经理')")
            .execute(&main_pool)
            .await
            .expect("insert role");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let butler_messages = insert_complete_user_messages(&conv_pool, &butler_conv.id).await;
        conversations::update_message_routing_metadata(
            &conv_pool,
            &butler_messages[0].id,
            r#"{"delegations":[{"targetRoleId":"role-1","targetRoleName":"产品经理","taskSummary":"准备设计评审","status":"ok"}]}"#,
        )
        .await
        .expect("write routing metadata");
        let role_conv = conversations::create_conversation(&conv_pool, Some("role-1"))
            .await
            .unwrap();
        let role_messages = insert_complete_user_messages(&conv_pool, &role_conv.id).await;

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(ScopedMemoryProvider::new(&role_messages[0].id)),
        )
        .await
        .expect("delegated role extraction succeeds");

        let stored: Vec<(Option<String>, String)> =
            sqlx::query_as("SELECT role_id, source_conversation_id FROM memories")
                .fetch_all(&main_pool)
                .await
                .expect("query memories");
        assert_eq!(count, 1);
        assert_eq!(stored, vec![(Some("role-1".to_string()), role_conv.id)]);
    }

    #[tokio::test]
    async fn extract_for_conversation_excludes_delegated_turn_assistant_messages_from_global_memory(
    ) {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品经理')")
            .execute(&main_pool)
            .await
            .expect("insert role");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let delegated_user = conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "请产品经理准备设计评审",
            true,
        )
        .await
        .expect("insert delegated user message");
        conversations::update_message_routing_metadata(
            &conv_pool,
            &delegated_user.id,
            r#"{"delegations":[{"targetRoleId":"role-1","targetRoleName":"产品经理","taskSummary":"准备设计评审","status":"ok"}]}"#,
        )
        .await
        .expect("write routing metadata");
        let assistant = conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "assistant",
            "产品经理建议调整设计评审。",
            true,
        )
        .await
        .expect("insert assistant summary");
        conversations::insert_message(&conv_pool, &butler_conv.id, "user", "继续记录偏好 1", true)
            .await
            .expect("insert follow-up user message");
        conversations::insert_message(&conv_pool, &butler_conv.id, "user", "继续记录偏好 2", true)
            .await
            .expect("insert follow-up user message");

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(DelegatedTurnLeakProvider::new(&assistant.id)),
        )
        .await
        .expect("global extraction succeeds");

        let stored_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(&main_pool)
            .await
            .expect("count memories");
        assert_eq!(count, 0);
        assert_eq!(stored_count, 0);
    }

    #[tokio::test]
    async fn extract_for_conversation_extracts_single_turn_delegated_conversation_by_recorded_id() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品经理')")
            .execute(&main_pool)
            .await
            .expect("insert role");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let butler_message = conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "请产品经理准备设计评审",
            true,
        )
        .await
        .expect("insert butler delegated message");
        let old_role_conv = conversations::create_conversation(&conv_pool, Some("role-1"))
            .await
            .unwrap();
        let old_role_messages = insert_complete_user_messages(&conv_pool, &old_role_conv.id).await;
        let delegated_role_conv = conversations::create_conversation(&conv_pool, Some("role-1"))
            .await
            .unwrap();
        let delegated_user = conversations::insert_message(
            &conv_pool,
            &delegated_role_conv.id,
            "user",
            "[管家委派] 准备设计评审",
            true,
        )
        .await
        .expect("insert delegated role user message");
        conversations::insert_message(
            &conv_pool,
            &delegated_role_conv.id,
            "assistant",
            "设计评审需要先确认用户旅程。",
            true,
        )
        .await
        .expect("insert delegated role assistant message");
        conversations::update_message_routing_metadata(
            &conv_pool,
            &butler_message.id,
            &format!(
                r#"{{"delegations":[{{"targetRoleId":"role-1","targetRoleName":"产品经理","targetConversationId":"{}","taskSummary":"准备设计评审","status":"ok"}}]}}"#,
                delegated_role_conv.id
            ),
        )
        .await
        .expect("write routing metadata");

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(RolePromptProvider::new(
                &old_role_messages[0].id,
                &old_role_messages[0].content,
                &delegated_user.id,
                &delegated_user.content,
            )),
        )
        .await
        .expect("delegated extraction succeeds");

        let stored: Vec<(Option<String>, String, String)> =
            sqlx::query_as("SELECT role_id, source_conversation_id, content FROM memories")
                .fetch_all(&main_pool)
                .await
                .expect("query memories");
        assert_eq!(count, 1);
        assert_eq!(
            stored,
            vec![(
                Some("role-1".to_string()),
                delegated_role_conv.id,
                "角色二最新委派记忆".to_string(),
            )]
        );
    }

    #[tokio::test]
    async fn extract_for_conversation_ignores_recorded_conversation_for_different_role() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query(
            "INSERT INTO roles (id, name) VALUES ('role-1', '产品经理'), ('role-2', '工程师')",
        )
        .execute(&main_pool)
        .await
        .expect("insert roles");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let butler_messages = insert_complete_user_messages(&conv_pool, &butler_conv.id).await;
        let wrong_role_conv = conversations::create_conversation(&conv_pool, Some("role-2"))
            .await
            .unwrap();
        let wrong_role_messages =
            insert_complete_user_messages(&conv_pool, &wrong_role_conv.id).await;
        conversations::update_message_routing_metadata(
            &conv_pool,
            &butler_messages[0].id,
            &format!(
                r#"{{"delegations":[{{"targetRoleId":"role-1","targetRoleName":"产品经理","targetConversationId":"{}","taskSummary":"准备设计评审","status":"ok"}}]}}"#,
                wrong_role_conv.id
            ),
        )
        .await
        .expect("write routing metadata");

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(RolePromptProvider::new(
                "missing-role-1-source",
                "",
                &wrong_role_messages[0].id,
                &wrong_role_messages[0].content,
            )),
        )
        .await
        .expect("delegated extraction succeeds");

        let stored_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(&main_pool)
            .await
            .expect("count memories");
        assert_eq!(count, 0);
        assert_eq!(stored_count, 0);
    }

    #[tokio::test]
    async fn extract_for_conversation_extracts_only_new_delegated_role_conversations() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query(
            "INSERT INTO roles (id, name) VALUES ('role-1', '产品经理'), ('role-2', '工程师')",
        )
        .execute(&main_pool)
        .await
        .expect("insert roles");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let old_delegated =
            conversations::insert_message(&conv_pool, &butler_conv.id, "user", "历史委派", true)
                .await
                .expect("insert old delegated message");
        conversations::update_message_routing_metadata(
            &conv_pool,
            &old_delegated.id,
            r#"{"delegations":[{"targetRoleId":"role-1","targetRoleName":"产品经理","taskSummary":"历史委派","status":"ok"}]}"#,
        )
        .await
        .expect("write old routing metadata");
        let latest_messages = insert_complete_user_messages(&conv_pool, &butler_conv.id).await;
        conversations::update_message_routing_metadata(
            &conv_pool,
            &latest_messages[0].id,
            r#"{"delegations":[{"targetRoleId":"role-2","targetRoleName":"工程师","taskSummary":"最新委派","status":"ok"}]}"#,
        )
        .await
        .expect("write latest routing metadata");
        let role_1_conv = conversations::create_conversation(&conv_pool, Some("role-1"))
            .await
            .unwrap();
        let role_1_messages = insert_complete_user_messages(&conv_pool, &role_1_conv.id).await;
        let role_2_conv = conversations::create_conversation(&conv_pool, Some("role-2"))
            .await
            .unwrap();
        let role_2_messages = insert_complete_user_messages(&conv_pool, &role_2_conv.id).await;

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(RolePromptProvider::new(
                &role_1_messages[0].id,
                &role_1_messages[0].content,
                &role_2_messages[0].id,
                &role_2_messages[0].content,
            )),
        )
        .await
        .expect("delegated extraction succeeds");

        let stored: Vec<(Option<String>, String, String)> =
            sqlx::query_as("SELECT role_id, source_conversation_id, content FROM memories")
                .fetch_all(&main_pool)
                .await
                .expect("query memories");
        assert_eq!(count, 1);
        assert_eq!(
            stored,
            vec![(
                Some("role-2".to_string()),
                role_2_conv.id,
                "角色二最新委派记忆".to_string(),
            ),]
        );
    }

    #[tokio::test]
    async fn extract_for_conversation_does_not_reuse_old_delegation_when_latest_delegation_fails() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query(
            "INSERT INTO roles (id, name) VALUES ('role-1', '产品经理'), ('role-2', '工程师')",
        )
        .execute(&main_pool)
        .await
        .expect("insert roles");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let old_delegated =
            conversations::insert_message(&conv_pool, &butler_conv.id, "user", "历史委派", true)
                .await
                .expect("insert old delegated message");
        conversations::update_message_routing_metadata(
            &conv_pool,
            &old_delegated.id,
            r#"{"delegations":[{"targetRoleId":"role-1","targetRoleName":"产品经理","taskSummary":"历史委派","status":"ok"}]}"#,
        )
        .await
        .expect("write old routing metadata");
        let latest_messages = insert_complete_user_messages(&conv_pool, &butler_conv.id).await;
        conversations::update_message_routing_metadata(
            &conv_pool,
            &latest_messages[0].id,
            r#"{"delegations":[{"targetRoleId":"role-2","targetRoleName":"工程师","taskSummary":"最新委派","status":"error"}]}"#,
        )
        .await
        .expect("write latest routing metadata");
        let role_1_conv = conversations::create_conversation(&conv_pool, Some("role-1"))
            .await
            .unwrap();
        let role_1_messages = insert_complete_user_messages(&conv_pool, &role_1_conv.id).await;
        let role_2_conv = conversations::create_conversation(&conv_pool, Some("role-2"))
            .await
            .unwrap();
        let role_2_messages = insert_complete_user_messages(&conv_pool, &role_2_conv.id).await;

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(RolePromptProvider::new(
                &role_1_messages[0].id,
                &role_1_messages[0].content,
                &role_2_messages[0].id,
                &role_2_messages[0].content,
            )),
        )
        .await
        .expect("delegated extraction succeeds");

        let stored_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(&main_pool)
            .await
            .expect("count memories");
        assert_eq!(count, 0);
        assert_eq!(stored_count, 0);
    }

    #[tokio::test]
    async fn extract_for_conversation_continues_role_extraction_when_global_json_fails() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品经理')")
            .execute(&main_pool)
            .await
            .expect("insert role");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let butler_messages = insert_complete_user_messages(&conv_pool, &butler_conv.id).await;
        conversations::update_message_routing_metadata(
            &conv_pool,
            &butler_messages[0].id,
            r#"{"delegations":[{"targetRoleId":"role-1","targetRoleName":"产品经理","taskSummary":"准备设计评审","status":"ok"}]}"#,
        )
        .await
        .expect("write routing metadata");
        let role_conv = conversations::create_conversation(&conv_pool, Some("role-1"))
            .await
            .unwrap();
        let role_messages = insert_complete_user_messages(&conv_pool, &role_conv.id).await;

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(GlobalErrorRolePromptProvider::new(&role_messages[0].id)),
        )
        .await
        .expect("role extraction continues");

        let stored: Vec<(Option<String>, String)> =
            sqlx::query_as("SELECT role_id, content FROM memories")
                .fetch_all(&main_pool)
                .await
                .expect("query memories");
        assert_eq!(count, 1);
        assert_eq!(
            stored,
            vec![(
                Some("role-1".to_string()),
                "角色记忆不应被全局错误阻断".to_string(),
            )]
        );
    }

    #[tokio::test]
    async fn extract_for_conversation_continues_other_roles_when_one_role_json_fails() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        sqlx::query(
            "INSERT INTO roles (id, name) VALUES ('role-1', '产品经理'), ('role-2', '工程师')",
        )
        .execute(&main_pool)
        .await
        .expect("insert roles");
        let butler_conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        let butler_messages = insert_complete_user_messages(&conv_pool, &butler_conv.id).await;
        conversations::update_message_routing_metadata(
            &conv_pool,
            &butler_messages[0].id,
            r#"{"delegations":[{"targetRoleId":"role-1","targetRoleName":"产品经理","taskSummary":"准备设计评审","status":"ok"},{"targetRoleId":"role-2","targetRoleName":"工程师","taskSummary":"评估实现","status":"ok"}]}"#,
        )
        .await
        .expect("write routing metadata");
        let role_1_conv = conversations::create_conversation(&conv_pool, Some("role-1"))
            .await
            .unwrap();
        let role_1_messages = insert_complete_user_messages(&conv_pool, &role_1_conv.id).await;
        let role_2_conv = conversations::create_conversation(&conv_pool, Some("role-2"))
            .await
            .unwrap();
        let role_2_messages = insert_complete_user_messages(&conv_pool, &role_2_conv.id).await;

        let count = extract_for_conversation_with_provider(
            &main_pool,
            &conv_pool,
            &butler_conv.id,
            Arc::new(RoleOneErrorRoleTwoOkProvider::new(
                &role_1_messages[0].id,
                &role_2_messages[0].id,
            )),
        )
        .await
        .expect("other role extraction continues");

        let stored: Vec<(Option<String>, String)> =
            sqlx::query_as("SELECT role_id, content FROM memories")
                .fetch_all(&main_pool)
                .await
                .expect("query memories");
        assert_eq!(count, 1);
        assert_eq!(
            stored,
            vec![(
                Some("role-2".to_string()),
                "角色二不应被角色一错误阻断".to_string(),
            )]
        );
    }

    #[tokio::test]
    async fn extract_with_provider_returns_empty_when_response_exceeds_limit() {
        let messages = vec![message("msg-1", "user", "请记住我的长期偏好")];
        let prompt = build_extraction_prompt(None, &messages);
        let extracted = extract_with_provider(
            Arc::new(FakeMemoryProvider::from_response(
                "x".repeat(MAX_EXTRACTION_RESPONSE_BYTES + 1),
            )),
            &prompt,
            &messages,
            "conv-1",
        )
        .await
        .expect("oversized response is swallowed");

        assert!(extracted.is_empty());
    }

    #[tokio::test]
    async fn extract_with_provider_drops_partial_response_after_stream_error() {
        let messages = vec![message("msg-1", "user", "请记住我的长期偏好")];
        let prompt = build_extraction_prompt(None, &messages);
        let extracted = extract_with_provider(
            Arc::new(ErrorAfterTokenProvider {
                token: r#"{"memories":[{"category":"fact","content":"不应写入","evidenceType":"explicit_statement","evidenceText":"请记住：我偏好中文沟通，并正在推进任务 0","sourceMessageIds":["msg-1"]}]}"#
                    .to_string(),
            }),
            &prompt,
            &messages,
            "conv-1",
        )
        .await
        .expect("stream errors are swallowed");

        assert!(extracted.is_empty());
    }

    #[tokio::test]
    async fn extract_for_conversation_returns_zero_when_provider_unavailable() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        let conv = conversations::create_conversation(&conv_pool, None)
            .await
            .unwrap();
        for i in 0..3 {
            conversations::insert_message(
                &conv_pool,
                &conv.id,
                "user",
                &format!("我希望系统记住偏好 {}", i),
                true,
            )
            .await
            .unwrap();
        }

        let count = extract_for_conversation(main_pool, conv_pool, conv.id)
            .await
            .expect("pipeline failures are swallowed");

        assert_eq!(count, 0);
    }
}
