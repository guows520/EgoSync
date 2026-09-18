/// 宿主事件发射接缝（Story 15.2 接缝二）。
///
/// 引擎内所有事件发射必须经此 trait；桌面壳提供 TauriEventBus 实现
/// （转发 `app_handle.emit`），未来 server 宿主提供 SSE 广播实现。
///
/// 设计约束（epic-15 冻结）：方法签名对象安全、非泛型——
/// `emit(event: &str, payload: serde_json::Value)`，禁泛型方法。
///
/// 错误形状为 `Result<(), String>`：宿主侧错误类型不能进 engine 契约，
/// 序列化/转发错误并入同一链，由各站点按既有语义（warn / ignore）处置。
pub trait EngineEvents: Send + Sync {
    /// 发射一条事件。`payload` 为已完成序列化的 JSON 值。
    fn emit(&self, event: &str, payload: serde_json::Value) -> Result<(), String>;
}
