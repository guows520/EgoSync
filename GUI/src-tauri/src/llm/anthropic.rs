use std::time::Duration;

use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;

use super::traits::{ChatCompletionMessage, ChatOptions, LlmProvider, StreamEvent, ToolCall};
use crate::error::AppError;

pub struct AnthropicProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(base_url: String, api_key: String, model: String) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| AppError::LlmError(format!("创建 HTTP 客户端失败: {}", e)))?;

        Ok(Self {
            base_url,
            api_key,
            model,
            client,
        })
    }

    fn messages_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/v1") {
            format!("{}/messages", base)
        } else {
            format!("{}/v1/messages", base)
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat_stream(
        &self,
        messages: Vec<ChatCompletionMessage>,
        tx: mpsc::Sender<StreamEvent>,
        options: ChatOptions,
    ) -> Result<(), AppError> {
        let url = self.messages_url();
        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(anthropic_message)
            .collect();

        let system_msg = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let mut body = json!({
            "model": self.model,
            "messages": msgs,
            "max_tokens": 4096,
            "stream": true
        });

        if let Some(sys) = system_msg {
            body["system"] = json!(sys);
        }

        if let Some(ref tools) = options.tools {
            let tools_json: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters
                    })
                })
                .collect();
            body["tools"] = json!(tools_json);

            if let Some(ref choice) = options.tool_choice {
                body["tool_choice"] = anthropic_tool_choice(choice);
            }
        }

        let stream_client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .http1_only()
            .build()
            .map_err(|e| AppError::LlmError(format!("创建流式客户端失败: {}", e)))?;

        let response = stream_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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
        let mut active_tool_id = String::new();
        let mut active_tool_name = String::new();
        let mut active_tool_input = String::new();

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
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        let event_type = parsed["type"].as_str().unwrap_or("");

                        match event_type {
                            "content_block_start" => {
                                let block = &parsed["content_block"];
                                if block["type"].as_str() == Some("tool_use") {
                                    active_tool_id = block["id"].as_str().unwrap_or("").to_string();
                                    active_tool_name =
                                        block["name"].as_str().unwrap_or("").to_string();
                                    active_tool_input.clear();
                                }
                            }
                            "content_block_delta" => {
                                let delta = &parsed["delta"];
                                match delta["type"].as_str().unwrap_or("") {
                                    "text_delta" => {
                                        if let Some(text) = delta["text"].as_str() {
                                            if !text.is_empty() {
                                                let _ = tx
                                                    .send(StreamEvent::Token(text.to_string()))
                                                    .await;
                                            }
                                        }
                                    }
                                    "input_json_delta" => {
                                        if let Some(partial) = delta["partial_json"].as_str() {
                                            active_tool_input.push_str(partial);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            "content_block_stop" => {
                                if !active_tool_name.is_empty() {
                                    let _ = tx
                                        .send(StreamEvent::ToolCall(ToolCall {
                                            id: active_tool_id.clone(),
                                            name: active_tool_name.clone(),
                                            arguments: active_tool_input.clone(),
                                        }))
                                        .await;
                                    active_tool_id.clear();
                                    active_tool_name.clear();
                                    active_tool_input.clear();
                                }
                            }
                            "message_stop" => {
                                let _ = tx.send(StreamEvent::Done).await;
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let _ = tx.send(StreamEvent::Done).await;
        Ok(())
    }

    async fn test_connection(&self) -> Result<(), AppError> {
        let url = self.messages_url();
        let body = json!({
            "model": self.model,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}]
        });

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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

fn anthropic_message(m: &ChatCompletionMessage) -> serde_json::Value {
    if m.role == "tool" {
        return json!({
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": m.tool_call_id.clone().unwrap_or_default(),
                "content": m.content
            }]
        });
    }

    if let Some(ref tool_calls) = m.tool_calls {
        let mut content = Vec::new();
        if !m.content.is_empty() {
            content.push(json!({"type": "text", "text": m.content}));
        }
        for tc in tool_calls {
            let input = serde_json::from_str::<serde_json::Value>(&tc.arguments)
                .unwrap_or_else(|_| json!({}));
            content.push(json!({
                "type": "tool_use",
                "id": tc.id,
                "name": tc.name,
                "input": input
            }));
        }
        return json!({"role": "assistant", "content": content});
    }

    json!({"role": m.role, "content": m.content})
}

fn anthropic_tool_choice(choice: &str) -> serde_json::Value {
    match choice {
        "required" => json!({"type": "any"}),
        "none" => json!({"type": "none"}),
        _ => json!({"type": "auto"}),
    }
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
    fn test_messages_url_with_v1() {
        let provider = AnthropicProvider::new(
            "https://api.anthropic.com/v1".to_string(),
            "sk-ant-test".to_string(),
            "claude-3-sonnet-20240229".to_string(),
        )
        .unwrap();
        assert_eq!(
            provider.messages_url(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn test_messages_url_without_v1() {
        let provider = AnthropicProvider::new(
            "https://api.anthropic.com".to_string(),
            "sk-ant-test".to_string(),
            "claude-3-sonnet-20240229".to_string(),
        )
        .unwrap();
        assert_eq!(
            provider.messages_url(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn test_anthropic_tool_result_message_maps_to_user_content_block() {
        let msg = ChatCompletionMessage {
            role: "tool".to_string(),
            content: "角色回复".to_string(),
            tool_calls: None,
            tool_call_id: Some("toolu_1".to_string()),
        };

        let mapped = anthropic_message(&msg);

        assert_eq!(mapped["role"], "user");
        assert_eq!(mapped["content"][0]["type"], "tool_result");
        assert_eq!(mapped["content"][0]["tool_use_id"], "toolu_1");
        assert_eq!(mapped["content"][0]["content"], "角色回复");
    }

    #[test]
    fn test_anthropic_assistant_tool_call_maps_to_tool_use_block() {
        let msg = ChatCompletionMessage {
            role: "assistant".to_string(),
            content: "稍等，我让产品经理看一下".to_string(),
            tool_calls: Some(vec![ToolCall {
                id: "toolu_1".to_string(),
                name: "delegate_to_role".to_string(),
                arguments: "{\"target_role_id\":\"role-1\",\"task_summary\":\"跟进 OKR\"}"
                    .to_string(),
            }]),
            tool_call_id: None,
        };

        let mapped = anthropic_message(&msg);

        assert_eq!(mapped["role"], "assistant");
        assert_eq!(mapped["content"][0]["type"], "text");
        assert_eq!(mapped["content"][1]["type"], "tool_use");
        assert_eq!(mapped["content"][1]["id"], "toolu_1");
        assert_eq!(mapped["content"][1]["name"], "delegate_to_role");
        assert_eq!(mapped["content"][1]["input"]["target_role_id"], "role-1");
    }

    #[test]
    fn test_anthropic_tool_choice_mapping() {
        assert_eq!(anthropic_tool_choice("required"), json!({"type": "any"}));
        assert_eq!(anthropic_tool_choice("none"), json!({"type": "none"}));
        assert_eq!(anthropic_tool_choice("auto"), json!({"type": "auto"}));
    }
}
