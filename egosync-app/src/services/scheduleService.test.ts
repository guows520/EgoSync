import { describe, expect, it, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { scheduleService } from './scheduleService';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe('scheduleService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getSchedule', () => {
    it('调用 settings_get_schedule 命令', async () => {
      mockInvoke.mockResolvedValue(null);

      await scheduleService.getSchedule();

      expect(mockInvoke).toHaveBeenCalledWith('settings_get_schedule');
    });

    it('返回 ScheduleConfig 对象', async () => {
      const config = {
        briefingTime: '07:30',
        reviewDay: '1',
        reviewTime: '19:00',
        bigrockReminderDay: '2',
        bigrockReminderTime: '10:00',
      };
      mockInvoke.mockResolvedValue(config);

      const result = await scheduleService.getSchedule();

      expect(result).toEqual(config);
    });
  });

  describe('updateSchedule', () => {
    it('调用 settings_update_schedule 命令并传递部分更新字段', async () => {
      mockInvoke.mockResolvedValue({
        briefingTime: '08:00',
        reviewDay: '3',
        reviewTime: '20:00',
        bigrockReminderDay: '1',
        bigrockReminderTime: '09:00',
      });

      await scheduleService.updateSchedule({ reviewDay: '3' });

      expect(mockInvoke).toHaveBeenCalledWith('settings_update_schedule', { reviewDay: '3' });
    });

    it('传递多个字段时全部传给后端', async () => {
      mockInvoke.mockResolvedValue({
        briefingTime: '07:00',
        reviewDay: '1',
        reviewTime: '18:00',
        bigrockReminderDay: '2',
        bigrockReminderTime: '09:00',
      });

      await scheduleService.updateSchedule({ briefingTime: '07:00', reviewTime: '18:00' });

      expect(mockInvoke).toHaveBeenCalledWith('settings_update_schedule', {
        briefingTime: '07:00',
        reviewTime: '18:00',
      });
    });

    it('返回更新后的完整 ScheduleConfig', async () => {
      const updated = {
        briefingTime: '08:00',
        reviewDay: '5',
        reviewTime: '20:00',
        bigrockReminderDay: '1',
        bigrockReminderTime: '09:00',
      };
      mockInvoke.mockResolvedValue(updated);

      const result = await scheduleService.updateSchedule({ reviewDay: '5' });

      expect(result).toEqual(updated);
    });

    it('命令失败时抛出错误', async () => {
      mockInvoke.mockRejectedValue(new Error('ValidationError'));

      await expect(scheduleService.updateSchedule({ reviewDay: '8' })).rejects.toThrow('ValidationError');
    });
  });
});
