import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TaskModal } from './TaskModal';
import type { Task } from '../../types/task';

const scope = { ownerType: 'role' as const, roleId: 'role-1' };

function sampleTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task-1',
    ownerType: 'role',
    roleId: 'role-1',
    title: '旧任务',
    deadline: null,
    quadrant: 'Q2',
    isBigRock: false,
    isCompleted: false,
    completedAt: null,
    sortOrder: 0,
    protectionStatus: 'normal',
    confidence: null,
    manualOverride: false,
    classificationReason: null,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
    deletedAt: null,
    ...overrides,
  };
}

describe('TaskModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('提交新任务时返回当前角色、标题、象限、截止日期和大石头标记', async () => {
    const onClose = vi.fn();
    const onSave = vi.fn().mockResolvedValue(undefined);

    render(<TaskModal scope={scope} onClose={onClose} onSave={onSave} />);

    fireEvent.change(screen.getByLabelText('任务内容'), { target: { value: '准备 Q3 OKR 规划' } });
    fireEvent.change(screen.getByLabelText('四象限分类'), { target: { value: 'Q1' } });
    fireEvent.change(screen.getByLabelText('截止时间'), { target: { value: '2026-06-30' } });
    fireEvent.click(screen.getByLabelText('标记为本周大石头'));
    fireEvent.click(screen.getByRole('button', { name: '保存任务' }));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        ownerType: 'role',
        roleId: 'role-1',
        title: '准备 Q3 OKR 规划',
        quadrant: 'Q1',
        deadline: '2026-06-30',
        isBigRock: true,
      });
    });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('编辑任务时预填表单并提交更新 payload', async () => {
    const onClose = vi.fn();
    const onSave = vi.fn().mockResolvedValue(undefined);

    render(
      <TaskModal
        scope={scope}
        onClose={onClose}
        onSave={onSave}
        task={sampleTask()}
      />,
    );

    expect(screen.getByRole('heading', { name: '编辑任务' })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('任务内容'), { target: { value: '更新任务' } });
    fireEvent.click(screen.getByRole('button', { name: '保存任务' }));

    // 仅改标题、未动 quadrant：payload 不应包含 quadrant，避免冻结自动分类与临期升 Q1
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        title: '更新任务',
        deadline: null,
        isBigRock: false,
      });
    });
  });

  it('编辑任务时改变四象限才发送 quadrant（触发手动覆盖）', async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);

    render(
      <TaskModal
        scope={scope}
        onClose={vi.fn()}
        onSave={onSave}
        task={sampleTask()}
      />,
    );

    fireEvent.change(screen.getByLabelText('四象限分类'), { target: { value: 'Q1' } });
    fireEvent.click(screen.getByRole('button', { name: '保存任务' }));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        title: '旧任务',
        deadline: null,
        quadrant: 'Q1',
        isBigRock: false,
      });
    });
  });

  it('标题为空时禁用保存', () => {
    const onSave = vi.fn();

    render(<TaskModal scope={scope} onClose={vi.fn()} onSave={onSave} />);

    const saveButton = screen.getByRole('button', { name: '保存任务' });
    expect(saveButton).toBeDisabled();

    fireEvent.click(saveButton);
    expect(onSave).not.toHaveBeenCalled();
  });

  it('新建任务默认选择「让系统自动判断」，提交 payload 不包含 quadrant', async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);

    render(<TaskModal scope={scope} onClose={vi.fn()} onSave={onSave} />);

    fireEvent.change(screen.getByLabelText('任务内容'), { target: { value: '让系统判断的任务' } });
    fireEvent.click(screen.getByRole('button', { name: '保存任务' }));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        ownerType: 'role',
        roleId: 'role-1',
        title: '让系统判断的任务',
        isBigRock: false,
      });
    });
  });

  it('编辑任务时清空截止时间会提交 null', async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);

    render(
      <TaskModal
        scope={scope}
        onClose={vi.fn()}
        onSave={onSave}
        task={sampleTask({ title: '带截止时间的任务', deadline: '2026-06-30', quadrant: 'Q1', isBigRock: true })}
      />,
    );

    fireEvent.change(screen.getByLabelText('截止时间'), { target: { value: '' } });
    fireEvent.click(screen.getByRole('button', { name: '保存任务' }));

    // 仅清空截止时间、未动 quadrant：payload 不应包含 quadrant
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        title: '带截止时间的任务',
        deadline: null,
        isBigRock: true,
      });
    });
  });

  it('后端返回 big_rock 限制错误时显示对应中文消息且弹窗保持打开', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const onClose = vi.fn();
    const onSave = vi.fn().mockRejectedValue({
      ValidationError: '每个角色每周最多 3 个大石头，请先取消一个再标记',
    });

    render(<TaskModal scope={scope} onClose={onClose} onSave={onSave} />);

    fireEvent.change(screen.getByLabelText('任务内容'), { target: { value: '第四个大石头' } });
    fireEvent.click(screen.getByLabelText('标记为本周大石头'));
    fireEvent.click(screen.getByRole('button', { name: '保存任务' }));

    await waitFor(() => {
      expect(screen.getByText('每个角色每周最多 3 个大石头，请先取消一个再标记')).toBeInTheDocument();
    });
    expect(onClose).not.toHaveBeenCalled();

    consoleSpy.mockRestore();
  });
});
