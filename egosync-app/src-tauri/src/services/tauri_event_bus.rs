use egosync_engine::services::event_bus::EngineEvents;
use tauri::Emitter;

/// Story 15.2 接缝二桌面侧：Tauri 事件总线的 EngineEvents 实现。
///
/// 转发 `app_handle.emit`；宿主错误转 String（engine 契约不感知 tauri 错误类型）。
/// payload 已是序列化完成的 JSON Value，`emit` 对 Value 的再次序列化为
/// 规范化 JSON——与原「强类型 struct 直接 emit」的载荷字节一致。
///
/// Story 15.3：加 Clone（AppHandle 内部即 Arc 语义）——spawn 的后台任务
/// 需要持总线克隆发射（chat 清理闭包 / generate_title），并使 lib.rs 的
/// scheduler 总线与 manage 的单实例统一（deferred-work 15.3 项）。
/// 泛型化 `<R = Wry>`：特征测试（OQ1 裁决 A）以 MockRuntime 注入同款总线，
/// 生产路径默认 Wry 不变。
pub struct TauriEventBus<R: tauri::Runtime = tauri::Wry> {
    app_handle: tauri::AppHandle<R>,
}

// 手写 Clone 而非 derive：derive 会给 impl 附加 `R: Clone` 约束，
// 而 tauri::Runtime 不要求 Clone（AppHandle 自身 Clone 即可共享）。
impl<R: tauri::Runtime> Clone for TauriEventBus<R> {
    fn clone(&self) -> Self {
        Self {
            app_handle: self.app_handle.clone(),
        }
    }
}

impl<R: tauri::Runtime> TauriEventBus<R> {
    pub fn new(app_handle: tauri::AppHandle<R>) -> Self {
        Self { app_handle }
    }
}

impl<R: tauri::Runtime> EngineEvents for TauriEventBus<R> {
    fn emit(&self, event: &str, payload: serde_json::Value) -> Result<(), String> {
        self.app_handle
            .emit(event, payload)
            .map_err(|e| e.to_string())
    }
}
