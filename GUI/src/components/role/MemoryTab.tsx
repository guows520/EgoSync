import { useEffect, useRef, useState } from 'react';
import { Clock } from 'lucide-react';
import { useMemories } from '../../hooks/useMemories';
import { memoryService } from '../../services/memoryService';
import type { MemoryCategory, MemorySourceMessage } from '../../types/memory';
import { cn } from '../../lib/utils';

interface MemoryTabProps {
  roleId: string | null;
  includeRoleMemories?: boolean;
  showOwnerLabel?: boolean;
  roleLabels?: Record<string, string>;
  /** 受控的类别筛选值。仅在同时传入 onCategoryChange 时生效。 */
  category?: MemoryCategory;
  /** 受控回调。传入后由父组件持有筛选状态（用于 badge 联动）；不传则组件内部自管。 */
  onCategoryChange?: (category: MemoryCategory | undefined) => void;
  onMemoryDeleted?: () => void;
}

const categoryLabels: Record<MemoryCategory, string> = {
  fact: '事实',
  preference: '偏好',
  cognition_update: '认知模式',
  task_status: '任务状态',
};

const categoryFilters: Array<{ value: MemoryCategory | null; label: string }> = [
  { value: null, label: '全部' },
  { value: 'fact', label: '事实' },
  { value: 'preference', label: '偏好' },
  { value: 'cognition_update', label: '认知模式' },
];

interface SourceState {
  isLoading: boolean;
  error: string | null;
  messages: MemorySourceMessage[] | null;
}

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

function roleLabel(role: string) {
  if (role === 'user') return '用户';
  if (role === 'assistant') return '助手';
  return role;
}

export function MemoryTab({
  roleId,
  includeRoleMemories = false,
  showOwnerLabel = false,
  roleLabels = {},
  category,
  onCategoryChange,
  onMemoryDeleted,
}: MemoryTabProps) {
  const isControlled = onCategoryChange !== undefined;
  const [internalCategory, setInternalCategory] = useState<MemoryCategory | undefined>();
  const selectedCategory = isControlled ? category : internalCategory;
  const setSelectedCategory = (next: MemoryCategory | undefined) => {
    if (isControlled) onCategoryChange?.(next);
    else setInternalCategory(next);
  };
  const [expandedMemoryId, setExpandedMemoryId] = useState<string | null>(null);
  const [sourceStates, setSourceStates] = useState<Record<string, SourceState>>({});
  const [confirmingMemoryId, setConfirmingMemoryId] = useState<string | null>(null);
  const [deletingMemoryId, setDeletingMemoryId] = useState<string | null>(null);
  const [deleteErrors, setDeleteErrors] = useState<Record<string, string>>({});
  const sourceRequestIds = useRef<Record<string, number>>({});
  const { memories, isLoading, error, refetch } = useMemories({
    roleId,
    includeRoleMemories,
    category: selectedCategory,
  });

  useEffect(() => {
    setExpandedMemoryId(null);
    setConfirmingMemoryId(null);
    setDeletingMemoryId(null);
    setDeleteErrors({});
  }, [includeRoleMemories, roleId, selectedCategory]);

  useEffect(() => {
    return () => {
      sourceRequestIds.current = {};
    };
  }, []);

  function toggleSource(memoryId: string) {
    const willExpand = expandedMemoryId !== memoryId;
    setExpandedMemoryId(willExpand ? memoryId : null);
    if (!willExpand || sourceStates[memoryId]?.messages || sourceStates[memoryId]?.isLoading) return;

    const requestId = (sourceRequestIds.current[memoryId] ?? 0) + 1;
    sourceRequestIds.current[memoryId] = requestId;
    setSourceStates(prev => ({
      ...prev,
      [memoryId]: { isLoading: true, error: null, messages: null },
    }));
    memoryService
      .getSourceMessages(memoryId)
      .then(messages => {
        if (sourceRequestIds.current[memoryId] !== requestId) return;
        setSourceStates(prev => ({
          ...prev,
          [memoryId]: { isLoading: false, error: null, messages },
        }));
      })
      .catch(e => {
        if (sourceRequestIds.current[memoryId] !== requestId) return;
        console.error('加载记忆来源失败:', e);
        // messages 保持 null，使下次展开可重新请求（区别于"成功但来源为空"）。
        setSourceStates(prev => ({
          ...prev,
          [memoryId]: { isLoading: false, error: '来源对话已不可用', messages: null },
        }));
      });
  }

  function openForgetConfirm(memoryId: string) {
    setConfirmingMemoryId(memoryId);
    setDeleteErrors(prev => {
      const next = { ...prev };
      delete next[memoryId];
      return next;
    });
  }

  function cancelForget(memoryId: string) {
    if (deletingMemoryId === memoryId) return;
    setConfirmingMemoryId(current => (current === memoryId ? null : current));
  }

  function finalizeForgotten(memoryId: string) {
    setConfirmingMemoryId(current => (current === memoryId ? null : current));
    setExpandedMemoryId(current => (current === memoryId ? null : current));
    delete sourceRequestIds.current[memoryId];
    setSourceStates(prev => {
      const next = { ...prev };
      delete next[memoryId];
      return next;
    });
    refetch();
    onMemoryDeleted?.();
  }

  async function confirmForget(memoryId: string) {
    if (deletingMemoryId) return;
    setDeletingMemoryId(memoryId);
    setDeleteErrors(prev => {
      const next = { ...prev };
      delete next[memoryId];
      return next;
    });

    try {
      await memoryService.delete(memoryId);
      finalizeForgotten(memoryId);
    } catch (e) {
      const alreadyGone =
        typeof e === 'object' && e !== null && 'NotFound' in (e as Record<string, unknown>);
      if (alreadyGone) {
        // 记忆已在后端被删除（并发删除 / 已不存在）——视为遗忘成功，清理残留卡片
        finalizeForgotten(memoryId);
      } else {
        console.error('删除记忆失败:', e);
        setDeleteErrors(prev => ({
          ...prev,
          [memoryId]: '这条记忆暂时没忘掉，稍后再试一下',
        }));
      }
    } finally {
      setDeletingMemoryId(current => (current === memoryId ? null : current));
    }
  }

  const emptyCopy = includeRoleMemories
    ? '还没有记忆，多聊几次，我会慢慢记住重要的事'
    : '还没有记忆，多和这个角色聊聊吧';
  const visibleMemories = memories.filter(memory => memory.category !== 'task_status');

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap gap-2" aria-label="记忆类别筛选">
        {categoryFilters.map(filter => {
          const selected = selectedCategory === (filter.value ?? undefined);
          return (
            <button
              key={filter.label}
              type="button"
              aria-pressed={selected}
              onClick={() => setSelectedCategory(filter.value ?? undefined)}
              className={cn(
                'px-3 py-1.5 rounded-full border text-[12px] font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-400 focus:ring-offset-1',
                selected
                  ? 'bg-indigo-50 border-indigo-200 text-indigo-700'
                  : 'bg-white border-slate-200 text-slate-500 hover:bg-slate-50 hover:text-slate-800'
              )}
            >
              {filter.label}
            </button>
          );
        })}
      </div>

      {isLoading && <div className="text-[14px] text-slate-500">正在加载记忆...</div>}
      {!isLoading && error && <div className="text-[14px] text-red-500">{error}</div>}
      {!isLoading && !error && visibleMemories.length === 0 && (
        <div className="text-[14px] text-slate-500">{emptyCopy}</div>
      )}

      {!isLoading && !error && visibleMemories.map(memory => {
        const sourceState = sourceStates[memory.id];
        const expanded = expandedMemoryId === memory.id;
        const ownerLabel = memory.roleId ? roleLabels[memory.roleId] ?? '未知角色' : '管家';
        const isConfirmingForget = confirmingMemoryId === memory.id;
        const isDeleting = deletingMemoryId === memory.id;
        const deleteError = deleteErrors[memory.id];
        return (
          <div key={memory.id} className="bg-white border border-slate-200 rounded-xl p-5 shadow-sm hover:shadow-md transition-shadow">
            <div className="flex justify-between items-start mb-3">
              <h4 className="text-[15px] font-medium text-slate-800 flex items-center gap-2">
                <span className="text-indigo-500">🧠</span> {categoryLabels[memory.category]}
                {showOwnerLabel && (
                  <span className="rounded-full border border-slate-200 bg-slate-50 px-2 py-0.5 text-[11px] font-medium text-slate-500">
                    {ownerLabel}
                  </span>
                )}
              </h4>
              <button
                type="button"
                onClick={() => openForgetConfirm(memory.id)}
                disabled={isDeleting}
                className={cn(
                  'text-[12px] border px-2.5 py-1 rounded-md transition-colors focus:outline-none focus:ring-2 focus:ring-red-300 focus:ring-offset-1',
                  isDeleting
                    ? 'text-slate-400 border-slate-200 cursor-not-allowed bg-slate-50'
                    : 'text-red-600 border-red-200 hover:bg-red-50 hover:border-red-300'
                )}
              >
                遗忘
              </button>
            </div>
            <p className="text-[14px] text-slate-600 leading-relaxed mb-4">{memory.content}</p>
            {isConfirmingForget && (
              <div className="mb-4 rounded-lg border border-red-100 bg-red-50/70 p-3 text-[13px] text-slate-700">
                <p className="leading-relaxed">确定要忘记这条吗？忘了就真忘了哦。原始对话还会留在历史里。</p>
                {deleteError && <p className="mt-2 text-red-600">{deleteError}</p>}
                <div className="mt-3 flex flex-wrap gap-2">
                  <button
                    type="button"
                    onClick={() => confirmForget(memory.id)}
                    disabled={isDeleting}
                    className={cn(
                      'rounded-md border px-3 py-1.5 text-[12px] font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-red-300 focus:ring-offset-1',
                      isDeleting
                        ? 'cursor-not-allowed border-red-100 bg-red-100 text-red-300'
                        : 'border-red-200 bg-white text-red-600 hover:bg-red-100'
                    )}
                  >
                    确认遗忘
                  </button>
                  <button
                    type="button"
                    onClick={() => cancelForget(memory.id)}
                    disabled={isDeleting}
                    className={cn(
                      'rounded-md border px-3 py-1.5 text-[12px] font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-slate-300 focus:ring-offset-1',
                      isDeleting
                        ? 'cursor-not-allowed border-slate-200 bg-slate-50 text-slate-300'
                        : 'border-slate-200 bg-white text-slate-600 hover:bg-slate-50'
                    )}
                  >
                    再想想
                  </button>
                </div>
              </div>
            )}
            <button
              type="button"
              onClick={() => toggleSource(memory.id)}
              aria-expanded={expanded}
              aria-controls={`memory-source-${memory.id}`}
              aria-label={`来源对话 ${formatMemoryTime(memory.createdAt)} ${expanded ? '收起' : '查看原文'}`}
              className="w-full bg-slate-50 border border-slate-100 rounded-md p-2.5 text-[12px] text-slate-500 flex items-start gap-2 hover:bg-slate-100 hover:text-slate-700 transition-colors cursor-pointer text-left focus:outline-none focus:ring-2 focus:ring-indigo-400 focus:ring-offset-1"
            >
              <Clock size={14} className="mt-0.5 shrink-0" />
              <span className="flex-1 min-w-0">
                <span className="flex items-center justify-between gap-3">
                  <span className="font-medium text-slate-600">来源对话</span>
                  <span className="text-indigo-500 text-[11px] shrink-0">{expanded ? '收起' : '查看原文'}</span>
                </span>
                <span className="block mt-1 text-[11px] text-slate-400">{formatMemoryTime(memory.createdAt)}</span>
              </span>
            </button>
            {expanded && (
              <div id={`memory-source-${memory.id}`} className="mt-3 border-l-2 border-indigo-300 pl-4 space-y-2.5 animate-in slide-in-from-top-2 duration-200">
                {sourceState?.isLoading && <div className="text-[13px] text-slate-500">正在加载来源原文...</div>}
                {sourceState?.error && <div className="text-[13px] text-slate-500">来源对话已不可用</div>}
                {sourceState?.messages?.length === 0 && !sourceState.error && (
                  <div className="text-[13px] text-slate-500">来源对话已不可用</div>
                )}
                {sourceState?.messages?.map(message => (
                  <div
                    key={message.id}
                    className={cn(
                      'rounded-lg border p-3 text-[13px]',
                      message.isSource
                        ? 'border-indigo-200 bg-indigo-50/70'
                        : 'border-slate-200 bg-white'
                    )}
                  >
                    <div className="flex items-center justify-between gap-3 mb-1.5 text-[11px] text-slate-400">
                      <span className="font-medium text-slate-600">{roleLabel(message.role)}</span>
                      <span>{formatMemoryTime(message.createdAt)}</span>
                    </div>
                    <p className="text-slate-700 leading-relaxed whitespace-pre-wrap">{message.content}</p>
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
