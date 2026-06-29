import { describe, expect, it, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { getLatest, generateNow } from './briefingService';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe('briefingService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getLatest', () => {
    it('调用 briefing_get_latest 命令', async () => {
      mockInvoke.mockResolvedValue(null);

      await getLatest();

      expect(mockInvoke).toHaveBeenCalledWith('briefing_get_latest');
    });

    it('返回 Briefing 对象', async () => {
      const briefing = {
        id: 'b1',
        content: '早上好！今天你有3个任务...',
        date: '2026-06-25',
        createdAt: '2026-06-25T08:00:00Z',
      };
      mockInvoke.mockResolvedValue(briefing);

      const result = await getLatest();

      expect(result).toEqual(briefing);
    });

    it('无简报时返回 null', async () => {
      mockInvoke.mockResolvedValue(null);

      const result = await getLatest();

      expect(result).toBeNull();
    });
  });

  describe('generateNow', () => {
    it('调用 briefing_generate_now 命令', async () => {
      mockInvoke.mockResolvedValue(true);

      await generateNow();

      expect(mockInvoke).toHaveBeenCalledWith('briefing_generate_now');
    });

    it('生成成功时返回 true', async () => {
      mockInvoke.mockResolvedValue(true);

      const result = await generateNow();

      expect(result).toBe(true);
    });

    it('当天已有简报时返回 false', async () => {
      mockInvoke.mockResolvedValue(false);

      const result = await generateNow();

      expect(result).toBe(false);
    });

    it('命令失败时抛出错误', async () => {
      mockInvoke.mockRejectedValue(new Error('LLM error'));

      await expect(generateNow()).rejects.toThrow('LLM error');
    });
  });
});
