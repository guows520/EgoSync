import { useEffect, useState } from 'react';
import { Clock } from 'lucide-react';
import { memoryService } from '../../services/memoryService';
import type { Memory } from '../../types/memory';

interface MemoryTabProps {
  roleId: string | null;
  includeRoleMemories?: boolean;
}

const categoryLabels: Record<Memory['category'], string> = {
  preference: '偏好',
  task_status: '任务状态',
  cognition_update: '认知更新',
  fact: '事实',
};

function formatMemoryTime(createdAt: string) {
  const date = new Date(createdAt);
  if (Number.isNaN(date.getTime())) return createdAt;
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function MemoryTab({ roleId, includeRoleMemories = false }: MemoryTabProps) {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [expandedMemoryId, setExpandedMemoryId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);
    const request = includeRoleMemories ? memoryService.listAll() : memoryService.list(roleId);
    request
      .then(list => {
        if (!cancelled) setMemories(list);
      })
      .catch(e => {
        console.error('加载记忆失败:', e);
        if (!cancelled) setError('加载记忆失败');
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [includeRoleMemories, roleId]);

  if (isLoading) {
    return <div className="text-[14px] text-slate-500">正在加载记忆...</div>;
  }

  if (error) {
    return <div className="text-[14px] text-red-500">{error}</div>;
  }

  if (memories.length === 0) {
    return <div className="text-[14px] text-slate-500">暂无记忆</div>;
  }

  return (
    <div className="space-y-4">
      {memories.map(memory => (
        <div key={memory.id} className="bg-white border border-slate-200 rounded-xl p-5 shadow-sm hover:shadow-md transition-shadow">
          <div className="flex justify-between items-start mb-3">
            <h4 className="text-[15px] font-medium text-slate-800 flex items-center gap-2">
              <span className="text-indigo-500">🧠</span> {categoryLabels[memory.category]}
            </h4>
            <button className="text-[12px] text-red-500 hover:bg-red-50 hover:border-red-200 border border-transparent px-2.5 py-1 rounded-md transition-colors">遗忘</button>
          </div>
          <p className="text-[14px] text-slate-600 leading-relaxed mb-4">{memory.content}</p>
          <button
            onClick={() => setExpandedMemoryId(expandedMemoryId === memory.id ? null : memory.id)}
            aria-expanded={expandedMemoryId === memory.id}
            aria-controls={`memory-source-${memory.id}`}
            aria-label={`来源对话 ${formatMemoryTime(memory.createdAt)} ${expandedMemoryId === memory.id ? '收起' : '查看原文'}`}
            className="w-full bg-slate-50 border border-slate-100 rounded-md p-2.5 text-[12px] text-slate-500 flex items-start gap-2 hover:bg-slate-100 hover:text-slate-700 transition-colors cursor-pointer text-left"
          >
            <Clock size={14} className="mt-0.5 shrink-0" />
            <span className="flex-1 min-w-0">
              <span className="flex items-center justify-between gap-3">
                <span className="font-medium text-slate-600">来源对话</span>
                <span className="text-indigo-500 text-[11px] shrink-0">{expandedMemoryId === memory.id ? '收起' : '查看原文'}</span>
              </span>
              <span className="block mt-1 text-[11px] text-slate-400">{formatMemoryTime(memory.createdAt)}</span>
            </span>
          </button>
          {expandedMemoryId === memory.id && (
            <div id={`memory-source-${memory.id}`} className="mt-3 border-l-2 border-indigo-300 pl-4 space-y-2.5 animate-in slide-in-from-top-2 duration-200">
              <div className="text-[13px] text-slate-500">来源原文将在后续故事中接入</div>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
