// Story 16.3 评审轮 2（U15）：壳命令失败文案提取器。
//
// Tauri invoke 失败时 reject 值是 **序列化 AppError 单键对象**
// （如 `{ ValidationError: "远程实例地址不能为空" }`——本仓既有测试的
// mock 形状为证），不是 Error 实例：`String(e)` 会呈现
// 「[object Object]」，keyring/校验的真实原因对用户不可见。
//
// 提取次序：Error 实例 ⇒ message；普通对象 ⇒ 首个字符串值（AppError
// 单键形态）；兜底 String(e)。认证/切换面五处 catch 统一经本函数取文案
// （此前 `e instanceof Error ? e.message : String(e)` 各自为政）。

/** 从任意 reject 值提取用户可读文案（AppError 单键对象 ⇒ 首值）。 */
export function toErrorMessage(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (e !== null && typeof e === 'object') {
    const first = Object.values(e as Record<string, unknown>)[0];
    if (typeof first === 'string' && first.length > 0) return first;
  }
  return String(e);
}
