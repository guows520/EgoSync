// web 版手机浏览器五症状自适应修复·症状⑤：聊天滚动容器行为钉孔（新建
// 文件——ChatStream.test.tsx 一行不改，冻结规矩「新测试一律新建文件」）。
//
// 2026-09-26 review F4/F5：症状⑤最终走路径 b——容器保留 scroll-smooth
// （来源消息居中 :238/:241 动画与滚轮平滑的载体，review F4 证实直接删类
// 会让 :241 赋值即时化、抢占 :238 的平滑滚动），仅挂载/更新贴底赋值临时
// 置 scrollBehavior='auto' 再恢复（症状⑤的「直接显示最底部」）。
// 本测试钉：
// - 滚动容器带 scroll-smooth（再删一次类 ⇒ :238/:241 平滑动画静默丧失）；
// - 自动贴底 effect 以 auto→赋值→恢复序列执行（jsdom 无布局滚动，滚动
//   效果本身不可断言，观测 CSSStyleDeclaration 原型上的写入序列）。

import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { ChatStream } from './ChatStream';
import { chatService } from '../../services/chatService';

vi.mock('../../services/chatService', () => ({
  chatService: {
    getButlerConversation: vi.fn(),
    getRoleConversation: vi.fn(),
    getConversation: vi.fn(),
    listConversations: vi.fn(),
    getHistory: vi.fn(),
    sendMessage: vi.fn(),
    deleteConversation: vi.fn(),
    newConversation: vi.fn(),
    stopStreaming: vi.fn(),
    pickWorkingDirectory: vi.fn(),
    getMessageProcessEvents: vi.fn(),
  },
}));

vi.mock('../../hooks/useEngineEvent', () => ({
  useEngineEvent: vi.fn(),
}));

vi.mock('../../services/skillService', () => ({
  skillService: {
    listSelectableForScope: vi.fn().mockResolvedValue([]),
  },
}));

const butlerConv = {
  id: 'conv-butler',
  roleId: null,
  title: '',
  startedAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

/** CSSStyleDeclaration.prototype 上 scrollBehavior 访问器的原型补丁记录器。 */
function patchScrollBehaviorRecorder() {
  const proto = Object.getPrototypeOf(document.createElement('div').style);
  const original = Object.getOwnPropertyDescriptor(proto, 'scrollBehavior');
  const seen: string[] = [];
  Object.defineProperty(proto, 'scrollBehavior', {
    configurable: true,
    get() { return (this as unknown as { __probeValue?: string }).__probeValue ?? ''; },
    set(value: string) {
      seen.push(value);
      (this as unknown as { __probeValue?: string }).__probeValue = value;
    },
  });
  return {
    seen,
    restore() {
      if (original) Object.defineProperty(proto, 'scrollBehavior', original);
      else delete (proto as unknown as Record<string, unknown>).scrollBehavior;
    },
  };
}

describe('ChatStream 滚动容器（web 移动自适应修复·症状⑤）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(chatService.getHistory).mockResolvedValue([]);
    vi.mocked(chatService.listConversations).mockResolvedValue([]);
    vi.mocked(chatService.getMessageProcessEvents).mockResolvedValue([]);
  });

  it('容器保留 scroll-smooth + 挂载贴底走 auto→赋值→恢复序列', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    const recorder = patchScrollBehaviorRecorder();

    try {
      render(<ChatStream role={null} />);
      await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());

      // 类钉孔：scroll-smooth 是 :238/:241 平滑动画与滚轮平滑的载体
      const container = screen.getByTestId('chat-scroll-container');
      expect(container).toHaveClass('scroll-smooth');

      // 贴底序列：置 auto → 赋值 scrollTop → 恢复原值（症状⑤即时落底）
      expect(recorder.seen.length).toBeGreaterThanOrEqual(2);
      expect(recorder.seen[0]).toBe('auto');
      expect(recorder.seen[1]).toBe('');

      // 期间 scrollTop 已赋值（jsdom 无布局，scrollHeight=0 ⇒ 值为 0）
      expect(container.scrollTop).toBe(0);
    } finally {
      recorder.restore();
    }
  });
});
