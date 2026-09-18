//! Story 15.3: 会话状态注册表 —— chat 域六组会话状态归位引擎。
//!
//! 六个 newtype 类型定义自壳 `commands/chat.rs` 逐字节平移（derive 保持）；
//! `ChatSessionRegistry` 以 pub 字段组合（避免方法化重写——锁粒度守恒的
//! 可审计性优先，消费者 `.0` 字段访问模式保持）。壳经 `pub use` 回引
//! 六类型，既有消费者零改动。
//!
//! 键全部为 conversation_id；全 tokio::sync::Mutex、无 Tauri 类型。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct OpencodeMcpScopeLock(pub Arc<Mutex<()>>);

#[derive(Default)]
pub struct StreamingState(pub Arc<Mutex<HashSet<String>>>);

#[derive(Default)]
pub struct CancelTokens(pub Arc<Mutex<HashMap<String, CancellationToken>>>);

#[derive(Default, Clone)]
pub struct OpencodeSessionState {
    pub active_session_id: String,
    pub sessions_by_directory: HashMap<String, String>,
}

#[derive(Default)]
pub struct OpencodeSessions(pub Arc<Mutex<HashMap<String, OpencodeSessionState>>>);

/// Tracks which conversations are in onboarding mode.
/// Key: conversation_id, Value: current onboarding step
#[derive(Default)]
pub struct OnboardingConversations(pub Arc<Mutex<HashMap<String, u8>>>);

#[derive(Default, Clone)]
pub struct MemoryExtractionState(pub Arc<Mutex<HashMap<String, CancellationToken>>>);

/// 六组会话状态的组合注册表：宿主单 manage 注入，命令层薄调用取字段/方法。
#[derive(Default)]
pub struct ChatSessionRegistry {
    pub opencode_mcp_scope_lock: OpencodeMcpScopeLock,
    pub streaming_state: StreamingState,
    pub cancel_tokens: CancelTokens,
    pub opencode_sessions: OpencodeSessions,
    pub onboarding_conversations: OnboardingConversations,
    pub memory_extraction: MemoryExtractionState,
}

impl ChatSessionRegistry {
    /// 迁自壳 chat.rs replace_memory_extraction_token（锁结构逐字节保持：
    /// replace 与 cancel 旧 token 同锁——锁粒度守恒不变量⑤）。
    pub async fn replace_memory_extraction_token(
        &self,
        conversation_id: &str,
    ) -> CancellationToken {
        let token = CancellationToken::new();
        let mut tokens = self.memory_extraction.0.lock().await;
        if let Some(old_token) = tokens.insert(conversation_id.to_string(), token.clone()) {
            old_token.cancel();
        }
        token
    }

    /// 迁自壳 chat.rs cancel_memory_extraction_token（锁结构逐字节保持）。
    pub async fn cancel_memory_extraction_token(
        &self,
        conversation_id: &str,
    ) -> Option<CancellationToken> {
        let mut tokens = self.memory_extraction.0.lock().await;
        let token = tokens.remove(conversation_id);
        if let Some(token) = &token {
            token.cancel();
        }
        token
    }

    /// 迁自壳 chat.rs remove_memory_extraction_token_if_active（锁结构逐字节保持：
    /// 已取消的旧 token 不得误删新 token）。
    pub async fn remove_memory_extraction_token_if_active(
        &self,
        conversation_id: &str,
        token: &CancellationToken,
    ) {
        let mut tokens = self.memory_extraction.0.lock().await;
        if token.is_cancelled() {
            return;
        }
        tokens.remove(conversation_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn replace_memory_extraction_token_cancels_previous_token() {
        // WHY: 新一轮记忆提取开始时必须取消上一轮，否则旧任务在空闲等待
        // 期满后会对同一会话做重复提取。
        let registry = ChatSessionRegistry::default();
        let first = registry.replace_memory_extraction_token("conv-1").await;
        let second = registry.replace_memory_extraction_token("conv-1").await;

        assert!(first.is_cancelled());
        assert!(!second.is_cancelled());
        assert_eq!(registry.memory_extraction.0.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn cancelled_memory_extraction_token_cannot_remove_new_token() {
        // WHY: 已取消的旧任务醒来清理时，若无条件 remove 会把新一轮任务的
        // token 一并删掉——新一轮将无法被取消。
        let registry = ChatSessionRegistry::default();
        let first = registry.replace_memory_extraction_token("conv-1").await;
        let _second = registry.replace_memory_extraction_token("conv-1").await;

        registry
            .remove_memory_extraction_token_if_active("conv-1", &first)
            .await;

        assert_eq!(registry.memory_extraction.0.lock().await.len(), 1);
    }
}
