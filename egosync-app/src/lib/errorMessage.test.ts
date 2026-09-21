// Story 16.3 评审轮 2（U15）：壳命令失败文案提取器测试。
//
// 钉死三种 reject 形态的提取（Tauri reject 值 = 序列化 AppError 单键
// 对象——非 Error 实例；`String(e)` 呈 [object Object] 即回归）。

import { describe, expect, it } from 'vitest';
import { toErrorMessage } from './errorMessage';

describe('toErrorMessage（壳命令 reject 值文案提取）', () => {
  it('AppError 单键对象（Tauri reject 实际形状）⇒ 首值（不是 [object Object]）', () => {
    expect(toErrorMessage({ ValidationError: '远程实例地址不能为空' })).toBe(
      '远程实例地址不能为空'
    );
    expect(toErrorMessage({ SidecarError: 'keyring 不可用' })).toBe('keyring 不可用');
  });

  it('Error 实例 ⇒ message', () => {
    expect(toErrorMessage(new Error('IPC 通道异常'))).toBe('IPC 通道异常');
  });

  it('字符串与其他形态 ⇒ 兜底 String(e)', () => {
    expect(toErrorMessage('裸字符串')).toBe('裸字符串');
    expect(toErrorMessage(undefined)).toBe('undefined');
    // 非字符串首值（罕见形态）⇒ 兜底 String 整体，不崩不静默
    expect(toErrorMessage({ code: 42 })).toBe(String({ code: 42 }));
  });
});
