import { invoke } from '@tauri-apps/api/core';
import type { AllTasksFilter, CreateTaskInput, CrossRoleTask, Task, UpdateTaskInput } from '../types/task';

export const taskService = {
  listByRole: (roleId: string) =>
    invoke<Task[]>('task_list_by_role', { roleId }),
  listButler: () =>
    invoke<Task[]>('task_list_butler'),
  listAll: (filter: AllTasksFilter = {}) =>
    invoke<CrossRoleTask[]>('task_list_all', {
      quadrant: filter.quadrant ?? null,
      isBigRock: filter.isBigRock ?? null,
    }),
  create: (input: CreateTaskInput) =>
    invoke<Task>('task_create', { input }),
  update: (id: string, input: UpdateTaskInput) =>
    invoke<Task>('task_update', { id, input }),
  delete: (id: string) =>
    invoke<void>('task_delete', { id }),
  reorder: (taskIds: string[]) =>
    invoke<void>('task_reorder', { taskIds }),
  toggleComplete: (id: string, isCompleted: boolean) =>
    invoke<Task>('task_toggle_complete', { taskId: id, isCompleted }),
  checkProtectionStatus: () =>
    invoke<number>('task_check_protection_status'),
};
