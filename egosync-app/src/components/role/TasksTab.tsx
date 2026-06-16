import { useState } from 'react';
import { Circle, Edit2, Plus, Trash2 } from 'lucide-react';
import { Modal } from '../layout/Modal';
import type { Task, TaskQuadrant } from '../../types/task';

interface TasksTabProps {
  role: { color?: string };
  tasks: Task[];
  isLoading: boolean;
  error: string | null;
  onOpenTask: (task: Task | null) => void;
  onDeleteTask: (id: string) => Promise<void> | void;
}

const quadrantLabels: Record<TaskQuadrant, string> = {
  Q1: 'Q1 · 重要且紧急',
  Q2: 'Q2 · 重要不紧急',
  Q3: 'Q3 · 紧急不重要',
  Q4: 'Q4 · 不重要不紧急',
};

export function TasksTab({ role, tasks, isLoading, error, onOpenTask, onDeleteTask }: TasksTabProps) {
  const [pendingDeleteTask, setPendingDeleteTask] = useState<Task | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState('');

  const handleConfirmDelete = async () => {
    if (!pendingDeleteTask) return;
    setIsDeleting(true);
    setDeleteError('');
    try {
      await onDeleteTask(pendingDeleteTask.id);
      setPendingDeleteTask(null);
    } catch (e) {
      console.error('删除任务失败:', e);
      setDeleteError('任务暂时删除失败，请稍后再试');
    } finally {
      setIsDeleting(false);
    }
  };

  if (isLoading) {
    return <div className="text-[13px] text-slate-500">任务加载中...</div>;
  }

  if (error) {
    return <div className="text-[13px] text-red-600">{error}</div>;
  }

  const activeStyle = role.color ? { color: role.color, borderColor: role.color } : undefined;
  const groupedTasks = tasks.reduce<Record<TaskQuadrant, Task[]>>(
    (groups, task) => {
      groups[task.quadrant].push(task);
      return groups;
    },
    { Q1: [], Q2: [], Q3: [], Q4: [] },
  );
  const quadrants = (Object.keys(quadrantLabels) as TaskQuadrant[]).filter(quadrant => groupedTasks[quadrant].length > 0);

  return (
    <>
      <div className="space-y-6">
      <div className="flex justify-end">
        <button
          onClick={() => onOpenTask(null)}
          className="inline-flex items-center gap-1.5 hover:bg-slate-200 px-2.5 py-1.5 rounded-md transition-colors text-[13px] font-medium"
          style={activeStyle}
        >
          <Plus size={16}/> 新增任务
        </button>
      </div>

      {tasks.length === 0 ? (
        <div className="rounded-xl border border-dashed border-slate-200 bg-white/70 p-6 text-center text-[13px] text-slate-500">
          还没有任务，先添加一个小目标吧
        </div>
      ) : (
        quadrants.map(quadrant => (
          <div key={quadrant}>
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-[12px] font-bold tracking-widest text-slate-400 uppercase">{quadrantLabels[quadrant]}</h3>
            </div>
            <div className="space-y-2.5">
              {groupedTasks[quadrant].map(task => (
                <div
                  key={task.id}
                  role="button"
                  tabIndex={0}
                  aria-label={`打开编辑 ${task.title}`}
                  onClick={() => onOpenTask(task)}
                  onKeyDown={event => {
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault();
                      onOpenTask(task);
                    }
                  }}
                  className="bg-white border border-slate-200 rounded-xl p-4 flex gap-3.5 shadow-sm group hover:border-indigo-300 hover:shadow-md transition-all cursor-pointer focus:outline-none focus:ring-2 focus:ring-indigo-500/20"
                >
                  <button
                    onClick={event => event.stopPropagation()}
                    className="text-slate-300 hover:text-emerald-500 transition-colors shrink-0 mt-0.5"
                    aria-label={`完成 ${task.title}`}
                  >
                    <Circle size={20} strokeWidth={2.5} />
                  </button>
                  <div className="flex-1 min-w-0">
                    <p className="text-[14.5px] text-slate-800 font-medium leading-snug">{task.title}</p>
                    <div className="flex items-center gap-2 mt-2.5">
                      {task.deadline && <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-red-50 text-red-600 border border-red-100">{task.deadline}</span>}
                      {task.isBigRock && <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-amber-50 text-amber-600 border border-amber-100">大石头</span>}
                    </div>
                  </div>
                  <div className="flex items-start gap-1 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity">
                    <button
                      onClick={event => {
                        event.stopPropagation();
                        onOpenTask(task);
                      }}
                      className="p-1.5 rounded-md text-slate-400 hover:text-indigo-600 hover:bg-indigo-50"
                      aria-label={`编辑 ${task.title}`}
                    >
                      <Edit2 size={15} />
                    </button>
                    <button
                      onClick={event => {
                        event.stopPropagation();
                        setDeleteError('');
                        setPendingDeleteTask(task);
                      }}
                      className="p-1.5 rounded-md text-slate-400 hover:text-red-600 hover:bg-red-50"
                      aria-label={`删除 ${task.title}`}
                    >
                      <Trash2 size={15} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))
      )}
      </div>
      {pendingDeleteTask && (
        <Modal onClose={() => !isDeleting && setPendingDeleteTask(null)} width="w-[420px]">
          <div className="p-6">
            <h2 className="text-[18px] font-semibold text-slate-800">确认删除任务？</h2>
            <p className="mt-3 text-[13px] leading-6 text-slate-500">
              删除后任务会从当前清单移除，但历史记录会保留。确定要删除“{pendingDeleteTask.title}”吗？
            </p>
            {deleteError && <p className="mt-4 text-[12px] text-red-600">{deleteError}</p>}
            <div className="mt-6 flex justify-end gap-3">
              <button
                onClick={() => setPendingDeleteTask(null)}
                disabled={isDeleting}
                className="px-4 py-2 border border-slate-200 text-slate-600 rounded-lg text-[13px] font-medium hover:bg-slate-50 disabled:opacity-60"
              >
                取消
              </button>
              <button
                onClick={handleConfirmDelete}
                disabled={isDeleting}
                className="px-4 py-2 bg-red-600 text-white rounded-lg text-[13px] font-medium hover:bg-red-700 disabled:opacity-60"
              >
                {isDeleting ? '删除中...' : '确认删除'}
              </button>
            </div>
          </div>
        </Modal>
      )}
    </>
  );
}
