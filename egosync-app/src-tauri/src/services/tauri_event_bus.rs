use egosync_engine::services::event_bus::EngineEvents;
use tauri::Emitter;

/// Story 15.2 接缝二桌面侧：Tauri 事件总线的 EngineEvents 实现。
///
/// 转发 `app_handle.emit`；宿主错误转 String（engine 契约不感知 tauri 错误类型）。
/// payload 已是序列化完成的 JSON Value，`emit` 对 Value 的再次序列化为
/// 规范化 JSON——与原「强类型 struct 直接 emit」的载荷字节一致。
pub struct TauriEventBus {
    app_handle: tauri::AppHandle,
}

impl TauriEventBus {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl EngineEvents for TauriEventBus {
    fn emit(&self, event: &str, payload: serde_json::Value) -> Result<(), String> {
        self.app_handle
            .emit(event, payload)
            .map_err(|e| e.to_string())
    }
}
