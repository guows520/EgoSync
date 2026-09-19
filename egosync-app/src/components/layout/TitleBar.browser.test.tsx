// Story 15.5 评审 G5：TitleBar 浏览器宿主退化分支的真实验证。
//
// vitest 全程挂 __TAURI_INTERNALS__ 全局桩（默认 Tauri 分支），浏览器
// 分支（普通标题栏：无窗口控制区、无 data-tauri-drag-region）此前零
// 断言。本文件删桩走浏览器分支渲染，finally 恢复全局桩。

import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { __resetTransportForTests } from '@/transport';
import { TitleBar } from './TitleBar';

describe('TitleBar 浏览器退化（宿主门控）', () => {
  const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

  beforeEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });
  afterEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
  });

  it('浏览器分支：仅标题文本——无窗口控制按钮、无拖拽区', () => {
    render(<TitleBar />);
    expect(screen.getByText('EgoSync')).toBeInTheDocument();
    expect(screen.queryByTitle('最小化')).not.toBeInTheDocument();
    expect(screen.queryByTitle('还原')).not.toBeInTheDocument();
    expect(screen.queryByTitle('最大化')).not.toBeInTheDocument();
    expect(screen.queryByTitle('关闭')).not.toBeInTheDocument();
    expect(document.querySelector('[data-tauri-drag-region]')).toBeNull();
  });
});
