import { invoke } from '@/transport';

export interface SchedulerTimes {
  moderate: string[];
  proactive: string[];
}

export const schedulerService = {
  getTimes: () => invoke<SchedulerTimes>('scheduler_get_times'),
  setTimes: (moderate: string[], proactive: string[]) =>
    invoke<SchedulerTimes>('scheduler_set_times', { moderate, proactive }),
};
