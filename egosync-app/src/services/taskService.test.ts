import { describe, expect, it, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { taskService } from './taskService';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe('taskService.checkQ2Reminders', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('调用 task_check_q2_reminders 命令', async () => {
    mockInvoke.mockResolvedValue(undefined);

    await taskService.checkQ2Reminders();

    expect(mockInvoke).toHaveBeenCalledWith('task_check_q2_reminders');
  });

  it('命令失败时抛出错误', async () => {
    mockInvoke.mockRejectedValue(new Error('DB error'));

    await expect(taskService.checkQ2Reminders()).rejects.toThrow('DB error');
  });
});
