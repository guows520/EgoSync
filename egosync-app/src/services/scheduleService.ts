import { invoke } from '@tauri-apps/api/core';

export interface ScheduleConfig {
  briefingTime: string;
  reviewDay: string;
  reviewTime: string;
  bigrockReminderDay: string;
  bigrockReminderTime: string;
}

export const scheduleService = {
  getSchedule: () => invoke<ScheduleConfig>('settings_get_schedule'),
  updateSchedule: (input: Partial<ScheduleConfig>) =>
    invoke<ScheduleConfig>('settings_update_schedule', input),
};
