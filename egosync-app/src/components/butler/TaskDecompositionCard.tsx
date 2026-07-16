import { useState } from 'react';
import { ListTree } from 'lucide-react';
import { getRoleIconComponent } from '../../lib/roleIcons';
import type { TaskDecompositionProposal } from '../../types/taskDecomposition';

interface TaskDecompositionCardProps {
  proposal: TaskDecompositionProposal;
  onAccept: (id: string) => Promise<void> | void;
  onKeepSingle: (id: string) => Promise<void> | void;
}

export function TaskDecompositionCard({
  proposal,
  onAccept,
  onKeepSingle,
}: TaskDecompositionCardProps) {
  const [action, setAction] = useState<'accept' | 'keep-single' | null>(null);
  const [error, setError] = useState<string | null>(null);
  const RoleIcon = getRoleIconComponent(proposal.roleIcon);

  const runAction = async (
    nextAction: 'accept' | 'keep-single',
    callback: (id: string) => Promise<void> | void,
  ) => {
    if (action) return;
    setAction(nextAction);
    setError(null);
    try {
      await callback(proposal.id);
    } catch {
      setAction(null);
      setError('处理失败，提案仍保留，可稍后重试。');
    }
  };

  return (
    <article
      role="article"
      aria-label={`任务拆分提案：${proposal.taskSummary}`}
      className="rounded-[10px] border border-indigo-200 bg-white p-4 shadow-sm"
    >
      <div className="flex items-start gap-3">
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-indigo-50 text-indigo-600">
          <ListTree size={17} />
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h3 className="text-[14px] font-medium text-slate-800">建议拆分为 {proposal.items.length} 个任务</h3>
            <span className="inline-flex items-center gap-1 text-[11px] text-slate-400">
              <RoleIcon size={12} />
              {proposal.roleName}
            </span>
          </div>
          <p className="mt-1 text-[12px] text-slate-500">原事项：{proposal.taskSummary}</p>
          <ol className="mt-3 space-y-2">
            {proposal.items.map((item, index) => (
              <li key={`${item.title}-${index}`} className="rounded-lg bg-slate-50 px-3 py-2 text-[12px] text-slate-700">
                <span>{index + 1}. {item.title}</span>
                {item.deadline && <span className="ml-2 text-slate-400">截止 {item.deadline}</span>}
              </li>
            ))}
          </ol>
          {error && <p role="alert" className="mt-2 text-[12px] text-red-600">{error}</p>}
          <div className="mt-3 flex justify-end gap-2">
            <button
              type="button"
              disabled={action !== null}
              onClick={() => void runAction('keep-single', onKeepSingle)}
              className="rounded-lg border border-slate-200 px-3 py-1.5 text-[13px] font-medium text-slate-600 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {action === 'keep-single' ? '处理中…' : '不要拆分'}
            </button>
            <button
              type="button"
              disabled={action !== null}
              onClick={() => void runAction('accept', onAccept)}
              className="rounded-lg bg-indigo-600 px-4 py-1.5 text-[13px] font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {action === 'accept' ? '创建中…' : '接受拆分'}
            </button>
          </div>
        </div>
      </div>
    </article>
  );
}
