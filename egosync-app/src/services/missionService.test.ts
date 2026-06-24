import { describe, expect, it, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { missionService } from './missionService';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe('missionService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('get', () => {
    it('调用 mission_get 命令', async () => {
      mockInvoke.mockResolvedValue(null);

      await missionService.get();

      expect(mockInvoke).toHaveBeenCalledWith('mission_get');
    });

    it('返回 Mission 对象', async () => {
      const mission = {
        id: 'singleton',
        content: '我的使命',
        format: 'free' as const,
        updatedAt: '2026-06-24T10:00:00Z',
      };
      mockInvoke.mockResolvedValue(mission);

      const result = await missionService.get();

      expect(result).toEqual(mission);
    });

    it('无记录时返回 null', async () => {
      mockInvoke.mockResolvedValue(null);

      const result = await missionService.get();

      expect(result).toBeNull();
    });
  });

  describe('update', () => {
    it('调用 mission_update 命令并传递正确参数', async () => {
      const mission = {
        id: 'singleton',
        content: '新使命',
        format: 'free',
        updatedAt: '2026-06-24T10:00:00Z',
      };
      mockInvoke.mockResolvedValue(mission);

      await missionService.update('新使命', 'free');

      expect(mockInvoke).toHaveBeenCalledWith('mission_update', {
        content: '新使命',
        format: 'free',
      });
    });

    it('structured 格式时传递 JSON content', async () => {
      const jsonContent = JSON.stringify({ role: '丈夫', value: '诚信', goal: '影响力' });
      const mission = {
        id: 'singleton',
        content: jsonContent,
        format: 'structured',
        updatedAt: '2026-06-24T10:00:00Z',
      };
      mockInvoke.mockResolvedValue(mission);

      await missionService.update(jsonContent, 'structured');

      expect(mockInvoke).toHaveBeenCalledWith('mission_update', {
        content: jsonContent,
        format: 'structured',
      });
    });

    it('空 content 传递 null', async () => {
      const mission = {
        id: 'singleton',
        content: null,
        format: 'free',
        updatedAt: '2026-06-24T10:00:00Z',
      };
      mockInvoke.mockResolvedValue(mission);

      await missionService.update(null, 'free');

      expect(mockInvoke).toHaveBeenCalledWith('mission_update', {
        content: null,
        format: 'free',
      });
    });

    it('命令失败时抛出错误', async () => {
      mockInvoke.mockRejectedValue(new Error('DB error'));

      await expect(missionService.update('内容', 'free')).rejects.toThrow('DB error');
    });
  });

  describe('inferValues', () => {
    it('调用 mission_infer 命令', async () => {
      mockInvoke.mockResolvedValue(null);

      await missionService.inferValues();

      expect(mockInvoke).toHaveBeenCalledWith('mission_infer');
    });

    it('返回 InferredValues 对象', async () => {
      const inferred = {
        values: ['家庭陪伴 > 工作效率 > 个人学习'],
        summary: '用户频繁优先处理家庭相关任务',
        confidence: 0.85,
      };
      mockInvoke.mockResolvedValue(inferred);

      const result = await missionService.inferValues();

      expect(result).toEqual(inferred);
    });

    it('数据不足时返回 null', async () => {
      mockInvoke.mockResolvedValue(null);

      const result = await missionService.inferValues();

      expect(result).toBeNull();
    });
  });
});
