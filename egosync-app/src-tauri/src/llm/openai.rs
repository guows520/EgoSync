use std::{collections::BTreeMap, time::Duration};

use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;

use super::traits::{ChatCompletionMessage, ChatOptions, LlmProvider, StreamEvent, ToolCall};
use crate::error::AppError;

pub struct OpenAiProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: Client,
    stream_client: Client,
    reasoning_split: bool,
}

impl OpenAiProvider {
    pub fn new(
        base_url: String,
        api_key: String,
        model: String,
        no_proxy: bool,
    ) -> Result<Self, AppError> {
        let mut builder = Client::builder().timeout(Duration::from_secs(10));
        if no_proxy {
            builder = builder.no_proxy();
        }
        let client = builder
            .build()
            .map_err(|e| AppError::LlmError(format!("创建 HTTP 客户端失败: {}", e)))?;
        let mut stream_builder = Client::builder().connect_timeout(Duration::from_secs(10)).http1_only();
        if no_proxy { stream_builder = stream_builder.no_proxy(); }
        let stream_client = stream_builder.build()
            .map_err(|e| AppError::LlmError(format!("创建流式客户端失败: {}", e)))?;

        Ok(Self {
            base_url,
            api_key,
            model,
            client,
            stream_client,
            reasoning_split: false,
        })
    }

    pub fn new_with_reasoning_split(
        base_url: String,
        api_key: String,
        model: String,
        no_proxy: bool,
    ) -> Result<Self, AppError> {
        let mut p = Self::new(base_url, api_key, model, no_proxy)?;
        p.reasoning_split = true;
        Ok(p)
    }

    fn completions_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}/chat/completions", base)
    }
}

fn serialize_messages(messages: &[ChatCompletionMessage]) -> Vec<serde_json::Value> {
    messages.iter().map(|m| {
        let mut msg = json!({"role": m.role, "content": m.content});
        if let Some(reasoning) = m.reasoning_content.as_ref().filter(|value| !value.trim().is_empty()) {
            msg["reasoning_content"] = json!(reasoning);
        }
        if let Some(ref tool_calls) = m.tool_calls {
            let tc: Vec<serde_json::Value> = tool_calls.iter().map(|tc| json!({
                "id": tc.id, "type": "function",
                "function": { "name": tc.name, "arguments": tc.arguments }
            })).collect();
            msg["tool_calls"] = json!(tc);
            if m.content.is_empty() { msg["content"] = json!(null); }
        }
        if let Some(ref tool_call_id) = m.tool_call_id { msg["tool_call_id"] = json!(tool_call_id); }
        msg
    }).collect()
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat_stream(
        &self,
        messages: Vec<ChatCompletionMessage>,
        tx: mpsc::Sender<StreamEvent>,
        options: ChatOptions,
    ) -> Result<(), AppError> {
        let url = self.completions_url();
        let msgs = serialize_messages(&messages);

        let mut body = json!({
            "model": self.model,
            "messages": msgs,
            "stream": true
        });

        if options.disable_thinking {
            body["enable_thinking"] = json!(false);
        }

        if self.reasoning_split {
            body["reasoning_split"] = json!(true);
        }

        if let Some(ref tools) = options.tools {
            let tools_json: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters
                        }
                    })
                })
                .collect();
            body["tools"] = json!(tools_json);

            // tool_choice: "auto" | "required" | "none"
            if let Some(ref tc) = options.tool_choice {
                body["tool_choice"] = json!(tc);
            }
        }

        let response = self.stream_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AppError::LlmError("连接超时，请检查网络或 Base URL 是否正确".to_string())
                } else if e.is_connect() {
                    AppError::LlmError(format!("无法连接到 {}，请检查 Base URL", self.base_url))
                } else {
                    AppError::LlmError(format!("网络请求失败: {}", e))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            let err_msg = match status.as_u16() {
                401 => "API Key 无效或已过期 (401 Unauthorized)".to_string(),
                403 => "API Key 权限不足 (403 Forbidden)".to_string(),
                404 => format!("模型 '{}' 不存在或端点不可用 (404)", self.model),
                429 => "请求频率过高或额度不足 (429)".to_string(),
                _ => format!(
                    "服务返回错误 (HTTP {}): {}",
                    status.as_u16(),
                    truncate_str(&body_text, 200)
                ),
            };
            let _ = tx.send(StreamEvent::Error(err_msg.clone())).await;
            return Err(AppError::LlmError(err_msg));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut pending_tool_calls: BTreeMap<usize, ToolCall> = BTreeMap::new();

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    let err_msg = format!("流式读取失败: {}", e);
                    let _ = tx.send(StreamEvent::Error(err_msg.clone())).await;
                    return Err(AppError::LlmError(err_msg));
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        emit_pending_tool_calls(&tx, &mut pending_tool_calls).await;
                        let _ = tx.send(StreamEvent::Done).await;
                        return Ok(());
                    }

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        let delta = &parsed["choices"][0]["delta"];

                        // Check for tool_calls in delta
                        if let Some(tool_calls) = delta["tool_calls"].as_array() {
                            for tc in tool_calls {
                                let index = tc["index"].as_u64().unwrap_or(0) as usize;
                                let pending =
                                    pending_tool_calls.entry(index).or_insert_with(|| ToolCall {
                                        id: String::new(),
                                        name: String::new(),
                                        arguments: String::new(),
                                    });

                                if let Some(id) = tc["id"].as_str() {
                                    pending.id = id.to_string();
                                }
                                if let Some(name) = tc["function"]["name"].as_str() {
                                    pending.name = name.to_string();
                                }
                                if let Some(args) = tc["function"]["arguments"].as_str() {
                                    pending.arguments.push_str(args);
                                }
                            }
                        }

                        // Check for finish_reason = "tool_calls"
                        if let Some(finish_reason) = parsed["choices"][0]["finish_reason"].as_str()
                        {
                            if finish_reason == "tool_calls" {
                                emit_pending_tool_calls(&tx, &mut pending_tool_calls).await;
                            }
                        }

                        // Regular content tokens
                        if let Some(reasoning) = delta["reasoning_content"]
                            .as_str()
                            .or_else(|| delta["reasoning"].as_str())
                            .or_else(|| delta["thinking"].as_str())
                        {
                            if !reasoning.is_empty() {
                                let _ = tx.send(StreamEvent::Thinking(reasoning.to_string())).await;
                            }
                        }
                        // MiniMax reasoning_split: thinking in reasoning_details array
                        if let Some(reasoning_details) = delta["reasoning_details"].as_array() {
                            for detail in reasoning_details {
                                if let Some(text) = detail["text"].as_str() {
                                    if !text.is_empty() {
                                        let _ = tx.send(StreamEvent::Thinking(text.to_string())).await;
                                    }
                                }
                            }
                        }
                        if let Some(content) = delta["content"].as_str() {
                            if !content.is_empty() {
                                let _ = tx.send(StreamEvent::Token(content.to_string())).await;
                            }
                        }
                    }
                }
            }
        }

        // If stream ended without [DONE], flush pending tool calls.
        emit_pending_tool_calls(&tx, &mut pending_tool_calls).await;

        let _ = tx.send(StreamEvent::Done).await;
        Ok(())
    }

    async fn test_connection(&self) -> Result<(), AppError> {
        let url = self.completions_url();
        let body = json!({
            "model": self.model,
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 1
        });

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AppError::LlmError("连接超时，请检查网络或 Base URL 是否正确".to_string())
                } else if e.is_connect() {
                    AppError::LlmError(format!("无法连接到 {}，请检查 Base URL", self.base_url))
                } else {
                    AppError::LlmError(format!("网络请求失败: {}", e))
                }
            })?;

        let status = response.status();
        if status.is_success() {
            return Ok(());
        }

        let body_text = response.text().await.unwrap_or_default();

        match status.as_u16() {
            401 => Err(AppError::LlmError(
                "API Key 无效或已过期 (401 Unauthorized)".to_string(),
            )),
            403 => Err(AppError::LlmError(
                "API Key 权限不足 (403 Forbidden)".to_string(),
            )),
            404 => Err(AppError::LlmError(format!(
                "模型 '{}' 不存在或端点不可用 (404 Not Found)",
                self.model
            ))),
            429 => Err(AppError::LlmError(
                "请求频率过高或额度不足 (429 Too Many Requests)".to_string(),
            )),
            _ => Err(AppError::LlmError(format!(
                "服务返回错误 (HTTP {}): {}",
                status.as_u16(),
                truncate_str(&body_text, 200)
            ))),
        }
    }
}

async fn emit_pending_tool_calls(
    tx: &mpsc::Sender<StreamEvent>,
    pending: &mut BTreeMap<usize, ToolCall>,
) {
    for tool_call in drain_pending_tool_calls(pending) {
        let _ = tx.send(StreamEvent::ToolCall(tool_call)).await;
    }
}

fn drain_pending_tool_calls(pending: &mut BTreeMap<usize, ToolCall>) -> Vec<ToolCall> {
    std::mem::take(pending)
        .into_values()
        .filter(|tool_call| !tool_call.name.is_empty())
        .collect()
}

fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completions_url_no_trailing_slash() {
        let provider = OpenAiProvider::new(
            "https://api.openai.com/v1".to_string(),
            "sk-test".to_string(),
            "gpt-4o".to_string(),
            false,
        )
        .unwrap();
        assert_eq!(
            provider.completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_completions_url_with_trailing_slash() {
        let provider = OpenAiProvider::new(
            "https://api.openai.com/v1/".to_string(),
            "sk-test".to_string(),
            "gpt-4o".to_string(),
            false,
        )
        .unwrap();
        assert_eq!(
            provider.completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_reasoning_content_is_serialized_for_tool_followup() {
        // WHY: DeepSeek requires the reasoning that produced a tool call on that assistant message.
        let messages = vec![ChatCompletionMessage { role: "assistant".into(), content: String::new(), reasoning_content: Some("分析后调用工具".into()), tool_calls: Some(vec![ToolCall { id: "call-1".into(), name: "create_role".into(), arguments: "{}".into() }]), tool_call_id: None }];
        let serialized = serialize_messages(&messages);
        assert_eq!(serialized[0]["reasoning_content"], "分析后调用工具");
    }

    #[test]
    fn test_empty_reasoning_content_is_omitted() {
        // WHY: messages without reasoning must retain their existing OpenAI-compatible wire shape.
        let messages = vec![ChatCompletionMessage { role: "user".into(), content: "你好".into(), reasoning_content: Some("  \n\t".into()), tool_calls: None, tool_call_id: None }];
        assert!(serialize_messages(&messages)[0].get("reasoning_content").is_none());
    }

    #[test]
    fn test_openai_tool_calls_are_drained_by_stream_index_order() {
        let mut pending = BTreeMap::new();
        pending.insert(
            1,
            ToolCall {
                id: "call-b".to_string(),
                name: "delegate_to_role".to_string(),
                arguments: "{\"target_role_id\":\"role-b\"}".to_string(),
            },
        );
        pending.insert(
            0,
            ToolCall {
                id: "call-a".to_string(),
                name: "delegate_to_role".to_string(),
                arguments: "{\"target_role_id\":\"role-a\"}".to_string(),
            },
        );

        let drained = drain_pending_tool_calls(&mut pending);

        assert!(
            pending.is_empty(),
            "finish_reason=tool_calls 后必须清空暂存，避免 [DONE] 重复执行委派"
        );
        assert_eq!(drained.len(), 2);
        assert_eq!(
            drained[0].id, "call-a",
            "多委派必须按 provider 给出的 index 顺序执行"
        );
        assert_eq!(drained[1].id, "call-b");
    }

    #[test]
    fn test_openai_tool_calls_skip_incomplete_entries() {
        let mut pending = BTreeMap::new();
        pending.insert(
            0,
            ToolCall {
                id: "call-a".to_string(),
                name: String::new(),
                arguments: "{}".to_string(),
            },
        );

        let drained = drain_pending_tool_calls(&mut pending);

        assert!(
            drained.is_empty(),
            "没有工具名的半截 delta 不能进入业务层执行"
        );
    }
}
