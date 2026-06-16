import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TaskModal } from './TaskModal';

const roleId = 'role-1';

describe('TaskModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('提交新任务时返回当前角色、标题、象限、截止日期和大石头标记', async () => {
    const onClose = vi.fn();
    const onSave = vi.fn().mockResolvedValue(undefined);

    render(<TaskModal roleId={roleId} onClose={onClose} onSave={onSave} />);

    fireEvent.change(screen.getByLabelText('任务内容'), { target: { value: '准备 Q3 OKR 规划' } });
    fireEvent.change(screen.getByLabelText('四象限分类'), { target: { value: 'Q1' } });
    fireEvent.change(screen.getByLabelText('截止时间'), { target: { value: '2026-06-30' } });
    fireEvent.click(screen.getByLabelText('标记为本周大石头'));
    fireEvent.click(screen.getByRole('button', { name: '保存任务' }));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        roleId,
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
        roleId={roleId}
        onClose={onClose}
        onSave={onSave}
        task={{
          id: 'task-1',
          roleId,
          title: '旧任务',
          deadline: null,
          quadrant: 'Q2',
          isBigRock: false,
          isCompleted: false,
          completedAt: null,
          sortOrder: 0,
          protectionStatus: 'normal',
          confidence: null,
          createdAt: '2026-06-01T00:00:00Z',
          updatedAt: '2026-06-01T00:00:00Z',
          deletedAt: null,
        }}
      />,
    );

    expect(screen.getByRole('heading', { name: '编辑任务' })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('任务内容'), { target: { value: '更新任务' } });
    fireEvent.click(screen.getByRole('button', { name: '保存任务' }));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        title: '更新任务',
        deadline: null,
        quadrant: 'Q2',
        isBigRock: false,
      });
    });
  });

  it('标题为空时禁用保存', () => {
    const onSave = vi.fn();

    render(<TaskModal roleId={roleId} onClose={vi.fn()} onSave={onSave} />);

    const saveButton = screen.getByRole('button', { name: '保存任务' });
    expect(saveButton).toBeDisabled();

    fireEvent.click(saveButton);
    expect(onSave).not.toHaveBeenCalled();
  });

  it('编辑任务时清空截止时间会提交 null', async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);

    render(
      <TaskModal
        roleId={roleId}
        onClose={vi.fn()}
        onSave={onSave}
        task={{
          id: 'task-1',
          roleId,
          title: '带截止时间的任务',
          deadline: '2026-06-30',
          quadrant: 'Q1',
          isBigRock: true,
          isCompleted: false,
          completedAt: null,
          sortOrder: 0,
          protectionStatus: 'normal',
          confidence: null,
          createdAt: '2026-06-01T00:00:00Z',
          updatedAt: '2026-06-01T00:00:00Z',
          deletedAt: null,
        }}
      />,
    );

    fireEvent.change(screen.getByLabelText('截止时间'), { target: { value: '' } });
    fireEvent.click(screen.getByRole('button', { name: '保存任务' }));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        title: '带截止时间的任务',
        deadline: null,
        quadrant: 'Q1',
        isBigRock: true,
      });
    });
  });
});
