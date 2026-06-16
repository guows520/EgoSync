import { useState } from 'react';
import { Target, X } from 'lucide-react';
import { Modal } from '../layout/Modal';
import type { CreateTaskInput, Task, TaskQuadrant, UpdateTaskInput } from '../../types/task';

interface TaskModalProps {
  roleId: string;
  task?: Task | null;
  onClose: () => void;
  onSave: (input: CreateTaskInput | UpdateTaskInput) => Promise<void> | void;
}

const quadrantOptions: Array<{ value: TaskQuadrant; label: string }> = [
  { value: 'Q1', label: 'Q1: 重要且紧急' },
  { value: 'Q2', label: 'Q2: 重要不紧急' },
  { value: 'Q3', label: 'Q3: 紧急不重要' },
  { value: 'Q4', label: 'Q4: 不重要不紧急' },
];

export function TaskModal({ roleId, task, onClose, onSave }: TaskModalProps) {
  const [title, setTitle] = useState(task?.title ?? '');
  const [quadrant, setQuadrant] = useState<TaskQuadrant>(task?.quadrant ?? 'Q1');
  const [deadline, setDeadline] = useState(task?.deadline ?? '');
  const [isBigRock, setIsBigRock] = useState(task?.isBigRock ?? false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState('');

  const isTitleBlank = title.trim().length === 0;

  const handleSubmit = async () => {
    const trimmedTitle = title.trim();
    if (!trimmedTitle) {
      setError('任务内容不能为空');
      return;
    }

    setIsSaving(true);
    setError('');
    try {
      if (task) {
        await onSave({
          title: trimmedTitle,
          deadline: deadline || null,
          quadrant,
          isBigRock,
        });
      } else {
        await onSave({
          roleId,
          title: trimmedTitle,
          quadrant,
          ...(deadline ? { deadline } : {}),
          isBigRock,
        });
      }
      onClose();
    } catch (e) {
      console.error('保存任务失败:', e);
      setError('任务暂时保存失败，请稍后再试');
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Modal onClose={onClose} width="w-[500px]">
      <div className="p-6">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-[18px] font-semibold text-slate-800">{task ? '编辑任务' : '新建任务'}</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600" aria-label="关闭任务弹窗"><X size={20}/></button>
        </div>
        
        <div className="space-y-5">
          <div>
            <label htmlFor="task-title" className="block text-[13px] font-medium text-slate-700 mb-1.5">任务内容</label>
            <input
              id="task-title"
              type="text"
              value={title}
              onChange={event => setTitle(event.target.value)}
              placeholder="例如：准备Q3 OKR规划"
              className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none"
            />
          </div>
          
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label htmlFor="task-quadrant" className="block text-[13px] font-medium text-slate-700 mb-1.5">四象限分类</label>
              <select
                id="task-quadrant"
                value={quadrant}
                onChange={event => setQuadrant(event.target.value as TaskQuadrant)}
                className="w-full bg-white border border-slate-200 rounded-lg px-3 py-2 text-[13px] focus:ring-2 focus:ring-indigo-500/20 outline-none"
              >
                {quadrantOptions.map(option => (
                  <option key={option.value} value={option.value}>{option.label}</option>
                ))}
              </select>
            </div>
            <div>
              <label htmlFor="task-deadline" className="block text-[13px] font-medium text-slate-700 mb-1.5">截止时间</label>
              <input
                id="task-deadline"
                type="date"
                value={deadline}
                onChange={event => setDeadline(event.target.value)}
                className="w-full bg-white border border-slate-200 rounded-lg px-3 py-2 text-[13px] focus:ring-2 focus:ring-indigo-500/20 outline-none text-slate-600"
              />
            </div>
          </div>

          <label className="flex items-center gap-3 p-3 border border-amber-200 bg-amber-50/50 rounded-lg cursor-pointer">
            <input
              type="checkbox"
              checked={isBigRock}
              onChange={event => setIsBigRock(event.target.checked)}
              aria-label="标记为本周大石头"
              className="w-4 h-4 text-amber-600 rounded border-amber-300 focus:ring-amber-500"
            />
            <div className="flex-1">
              <div className="text-[13px] font-semibold text-amber-800">标记为本周大石头</div>
              <div className="text-[11px] text-amber-600/80">在任务清单中作为本周重点展示</div>
            </div>
            <Target size={20} className="text-amber-500/50" />
          </label>
          {error && <p className="text-[12px] text-red-600">{error}</p>}
        </div>

        <div className="mt-8 flex justify-end gap-3">
          <button onClick={onClose} disabled={isSaving} className="px-5 py-2.5 border border-slate-200 text-slate-600 rounded-lg text-[13px] font-medium hover:bg-slate-50 disabled:opacity-60">取消</button>
          <button onClick={handleSubmit} disabled={isSaving || isTitleBlank} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700 disabled:opacity-60">保存任务</button>
        </div>
      </div>
    </Modal>
  );
}
