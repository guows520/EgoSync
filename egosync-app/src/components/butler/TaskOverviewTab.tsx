import { useState } from 'react';
import { AlertTriangle, CheckCircle2, ChevronDown, ChevronRight, Circle, Edit2, Filter, Loader2, Plus, Target, Trash2, Users } from 'lucide-react';
import { Modal } from '../layout/Modal';
import { cn, formatDeadline } from '../../lib/utils';
import type { CrossRoleTask, TaskQuadrant } from '../../types/task';
import type { Role } from '../../types/role';
import type { TaskScope } from '../../hooks/useTasks';

interface TaskOverviewTabProps {
  roles: Role[];
  tasks: CrossRoleTask[];
  isLoading: boolean;
  error: string | null;
  classifyingIds?: Set<string>;
  quadrantFilter: TaskQuadrant | 'all';
  onQuadrantFilterChange: (quadrant: TaskQuadrant | 'all') => void;
  showBigRocksOnly: boolean;
  onToggleBigRocksOnly: () => void;
  onOpenTask: (scope: TaskScope, task: CrossRoleTask | null) => void;
  onDeleteTask: (id: string) => Promise<void> | void;
  onToggleComplete: (id: string, isCompleted: boolean) => Promise<void> | void;
}

const EMPTY_CLASSIFYING_IDS: Set<string> = new Set();

const quadrantChips: Array<{ value: TaskQuadrant | 'all'; label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'Q1', label: 'Q1' },
  { value: 'Q2', label: 'Q2' },
  { value: 'Q3', label: 'Q3' },
  { value: 'Q4', label: 'Q4' },
];

function ownerKey(task: CrossRoleTask): string {
  return task.ownerType === 'butler' ? 'butler' : task.roleId ?? 'unknown';
}

const quadrantLabels: Record<TaskQuadrant, string> = {
  Q1: 'Q1 · 重要且紧急',
  Q2: 'Q2 · 重要不紧急',
  Q3: 'Q3 · 紧急不重要',
  Q4: 'Q4 · 不重要不紧急',
};

const quadrantTitleColor: Record<TaskQuadrant, string> = {
  Q1: 'text-slate-900',
  Q2: 'text-slate-900',
  Q3: 'text-slate-900',
  Q4: 'text-slate-900',
};

const quadrantEmptyHint: Record<TaskQuadrant, string> = {
  Q1: '没有紧急任务，太棒了！',
  Q2: '暂无重要规划，别忘了为长远目标留出时间',
  Q3: '没有需要应付的杂事，很清爽',
  Q4: '没有可有可无的任务，注意力很集中',
};

function ownerLabel(task: CrossRoleTask) {
  if (task.ownerType === 'butler') return '管家';
  return task.roleName ?? '未知角色';
}

function ownerColor(task: CrossRoleTask) {
  return task.ownerType === 'butler' ? '#6366F1' : task.roleColor ?? '#64748B';
}

function TaskCard({
  task,
  isClassifying,
  onOpenTask,
  onToggleComplete,
  onRequestDelete,
}: {
  task: CrossRoleTask;
  isClassifying: boolean;
  onOpenTask: (task: CrossRoleTask) => void;
  onToggleComplete: (task: CrossRoleTask) => void;
  onRequestDelete: (task: CrossRoleTask) => void;
}) {
  const color = ownerColor(task);
  return (
    <div className={cn(
      'bg-white border border-slate-200 rounded-xl p-4 flex gap-3.5 shadow-sm group hover:border-indigo-300 hover:shadow-md transition-all duration-200 motion-reduce:transition-none',
      task.isCompleted && 'opacity-60 scale-[0.99]',
      task.protectionStatus === 'at_risk' && 'border-l-4 border-l-amber-400',
    )}>
      <button
        onClick={event => {
          event.stopPropagation();
          onToggleComplete(task);
        }}
        className={cn(
          'transition-colors shrink-0 mt-0.5',
          task.isCompleted ? 'text-emerald-500' : 'text-slate-300 hover:text-emerald-500',
        )}
        aria-label={`${task.isCompleted ? '撤销完成' : '完成'} ${task.title}`}
      >
        {task.isCompleted ? <CheckCircle2 size={20} strokeWidth={2.5} /> : <Circle size={20} strokeWidth={2.5} />}
      </button>
      <div className="flex-1 min-w-0">
        <div>
          <p
            className={cn(
              'text-[14.5px] font-medium leading-snug',
              task.isCompleted ? 'text-slate-400 line-through' : 'text-slate-800',
            )}
          >
            {task.title}
          </p>
        </div>
        <div className="flex items-center gap-2 mt-2.5 flex-wrap">
          {task.deadline && <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-transparent text-slate-500 border border-slate-300">{formatDeadline(task.deadline)}</span>}
          {task.isBigRock && <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-transparent text-amber-600 border border-amber-400">大石头</span>}
          {task.protectionStatus === 'at_risk' && (
            <span
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-semibold bg-amber-50 text-amber-600 border border-amber-100"
              aria-label="重要任务被持续挤压，建议尽快处理"
            >
              <AlertTriangle size={11} />
              被挤压
            </span>
          )}
          {isClassifying && (
            <span
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-semibold bg-indigo-50 text-indigo-600 border border-indigo-100"
              aria-label="正在智能分类"
            >
              <Loader2 size={11} className="animate-spin motion-reduce:animate-none" />
              智能分类中…
            </span>
          )}
        </div>
      </div>
      <div className="shrink-0 self-center flex items-center justify-end" title={ownerLabel(task)}>
        <div className="flex items-center gap-1.5" style={{ width: '64px', justifyContent: 'flex-start' }}>
          <span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: color }} />
          <span className="text-[11px] font-semibold text-slate-600 whitespace-nowrap truncate">{ownerLabel(task)}</span>
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
            onRequestDelete(task);
          }}
          className="p-1.5 rounded-md text-slate-400 hover:text-red-600 hover:bg-red-50"
          aria-label={`删除 ${task.title}`}
        >
          <Trash2 size={15} />
        </button>
      </div>
    </div>
  );
}

export function TaskOverviewTab({ roles, tasks, isLoading, error, classifyingIds = EMPTY_CLASSIFYING_IDS, quadrantFilter, onQuadrantFilterChange, showBigRocksOnly, onToggleBigRocksOnly, onOpenTask, onDeleteTask, onToggleComplete }: TaskOverviewTabProps) {
  const [deselectedOwners, setDeselectedOwners] = useState<Set<string>>(() => new Set());
  const [pendingDeleteTask, setPendingDeleteTask] = useState<CrossRoleTask | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState('');
  const [actionError, setActionError] = useState('');
  const [expandedCompleted, setExpandedCompleted] = useState<Record<TaskQuadrant, boolean>>({
    Q1: false,
    Q2: false,
    Q3: false,
    Q4: false,
  });

  const visibleQuadrants = (quadrantFilter === 'all'
    ? (Object.keys(quadrantLabels) as TaskQuadrant[])
    : [quadrantFilter]);

  const ownerOptions: Array<{ key: string; label: string; color: string }> = [
    { key: 'butler', label: '管家', color: '#6366F1' },
    ...roles.map(role => ({ key: role.id, label: role.name, color: role.color })),
  ];

  const toggleOwner = (key: string) => {
    setDeselectedOwners(prev => {
      if (prev.size === 0) {
        return new Set(ownerOptions.filter(o => o.key !== key).map(o => o.key));
      }
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const toggleAllOwners = () => {
    setDeselectedOwners(prev => (
      prev.size === 0 ? new Set(ownerOptions.map(owner => owner.key)) : new Set()
    ));
  };

  // 象限 / 大石头已在服务端过滤；此处仅做角色（归属）多选的前端过滤。
  const filteredTasks = tasks.filter(task => !deselectedOwners.has(ownerKey(task)));
  const isDefaultFilter = quadrantFilter === 'all' && !showBigRocksOnly && deselectedOwners.size === 0;

  const groupedTasks = filteredTasks.reduce<Record<TaskQuadrant, CrossRoleTask[]>>(
    (groups, task) => {
      groups[task.quadrant].push(task);
      return groups;
    },
    { Q1: [], Q2: [], Q3: [], Q4: [] },
  );

  const orderByOverview = (a: CrossRoleTask, b: CrossRoleTask) =>
    (Number(b.isBigRock) - Number(a.isBigRock))
    || ownerLabel(a).localeCompare(ownerLabel(b), 'zh-CN')
    || (a.sortOrder - b.sortOrder);

  const incompleteByQuadrant = {} as Record<TaskQuadrant, CrossRoleTask[]>;
  const completedByQuadrant = {} as Record<TaskQuadrant, CrossRoleTask[]>;
  visibleQuadrants.forEach(quadrant => {
    const group = groupedTasks[quadrant] ?? [];
    incompleteByQuadrant[quadrant] = group.filter(task => !task.isCompleted).sort(orderByOverview);
    completedByQuadrant[quadrant] = group.filter(task => task.isCompleted).sort(orderByOverview);
  });

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

  const handleToggle = (task: CrossRoleTask) => {
    setActionError('');
    void Promise.resolve(onToggleComplete(task.id, !task.isCompleted)).catch(e => {
      console.error('切换任务完成状态失败:', e);
      setActionError('任务状态暂时切换失败，请稍后再试');
    });
  };

  const toggleCompletedSection = (quadrant: TaskQuadrant) => {
    setExpandedCompleted(prev => ({ ...prev, [quadrant]: !prev[quadrant] }));
  };

  if (isLoading) {
    return <div className="text-[13px] text-slate-500">任务加载中...</div>;
  }

  if (error) {
    return <div className="text-[13px] text-red-600">{error}</div>;
  }

  return (
    <>
      <div className="space-y-6">
        <div className="flex flex-col gap-3">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h3 className="text-[15px] font-semibold text-slate-800">全部任务概览</h3>
              <p className="mt-0.5 text-[12px] text-slate-500">汇总管家与所有角色的任务</p>
            </div>
            <button
              type="button"
              onClick={() => onOpenTask({ ownerType: 'butler' }, null)}
              className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-md transition-colors text-[13px] font-medium text-indigo-600 hover:bg-indigo-50"
            >
              <Plus size={16}/> 新增任务
            </button>
          </div>

          <div className="flex flex-col gap-2.5 rounded-xl border border-slate-200 bg-white/70 px-3 py-2.5">
            <div className="flex items-center gap-1 flex-wrap">
              <Filter size={14} className="text-slate-400" />
              {quadrantChips.map(chip => (
                <button
                  key={chip.value}
                  type="button"
                  onClick={() => onQuadrantFilterChange(chip.value)}
                  aria-pressed={quadrantFilter === chip.value}
                  className={cn(
                    'px-2 py-1 rounded-md text-[12px] font-medium transition-colors motion-reduce:transition-none',
                    quadrantFilter === chip.value ? 'bg-indigo-100 text-indigo-700' : 'text-slate-500 hover:bg-slate-100',
                  )}
                >
                  {chip.label}
                </button>
              ))}
              <span className="w-px h-4 bg-slate-200 mx-0.5" />
              <button
                type="button"
                onClick={onToggleBigRocksOnly}
                aria-pressed={showBigRocksOnly}
                className={cn(
                  'inline-flex items-center gap-1 px-1.5 py-1 rounded-md text-[12px] font-medium transition-colors motion-reduce:transition-none',
                  showBigRocksOnly ? 'bg-amber-100 text-amber-700' : 'text-slate-500 hover:bg-slate-100',
                )}
              >
                <Target size={13} /> 只看大石头
              </button>
            </div>
            <div className="flex items-start gap-1.5 min-w-0">
              <Users size={14} className="text-slate-400 shrink-0 mt-1.5" />
              <div className="thin-horizontal-scrollbar flex items-center gap-1.5 overflow-x-auto whitespace-nowrap pb-2 -mb-1" role="group" aria-label="按归属筛选">
                <button
                  type="button"
                  onClick={toggleAllOwners}
                  aria-pressed={deselectedOwners.size === 0}
                  className={cn(
                    'shrink-0 px-2.5 py-1 rounded-md text-[12px] font-medium transition-colors motion-reduce:transition-none',
                    deselectedOwners.size === 0 ? 'bg-indigo-100 text-indigo-700' : 'text-slate-500 hover:bg-slate-100',
                  )}
                >
                  全部
                </button>
                {ownerOptions.map(owner => {
                  const selected = deselectedOwners.size > 0 && !deselectedOwners.has(owner.key);
                  return (
                    <button
                      key={owner.key}
                      type="button"
                      onClick={() => toggleOwner(owner.key)}
                      aria-pressed={selected}
                      className={cn(
                        'shrink-0 inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[12px] font-medium transition-colors motion-reduce:transition-none',
                        selected ? 'bg-indigo-100 text-indigo-700' : 'text-slate-500 hover:bg-slate-100',
                      )}
                    >
                      <span className="w-2 h-2 rounded-full" style={{ backgroundColor: selected ? owner.color : '#CBD5E1' }} />
                      {owner.label}
                    </button>
                  );
                })}
              </div>
            </div>
          </div>
        </div>

        {actionError && <div role="alert" className="text-[13px] text-red-600">{actionError}</div>}

        {filteredTasks.length === 0 ? (
          <div className="rounded-xl border border-dashed border-slate-200 bg-white/70 p-6 text-center text-[13px] text-slate-500">
            {isDefaultFilter ? '所有角色都很轻松，可以考虑添加新目标' : '当前筛选无匹配任务'}
          </div>
        ) : (
          <div className="space-y-6">
            {visibleQuadrants.map(quadrant => {
              const count = groupedTasks[quadrant].length;
              const completedTasks = completedByQuadrant[quadrant];
              const isExpanded = expandedCompleted[quadrant];

              return (
                <section key={quadrant}>
                  <div className="flex items-center justify-between w-full mb-3">
                    <span className="flex items-center gap-1.5">
                      <span className={cn('text-[12px] font-bold tracking-widest', quadrantTitleColor[quadrant])}>
                        {quadrantLabels[quadrant]}
                      </span>
                    </span>
                    <span className="inline-flex items-center justify-center min-w-[20px] px-1.5 rounded-full bg-slate-100 text-slate-500 text-[11px] font-semibold">
                      {count}
                    </span>
                  </div>
                  <div className="space-y-2.5">
                    {count === 0 ? (
                      <p className="text-[12.5px] text-slate-400">{quadrantEmptyHint[quadrant]}</p>
                    ) : (
                      <>
                          {incompleteByQuadrant[quadrant].map(task => (
                            <TaskCard
                              key={task.id}
                              task={task}
                              isClassifying={classifyingIds.has(task.id)}
                              onOpenTask={task => onOpenTask({ ownerType: 'butler' }, task)}
                              onToggleComplete={handleToggle}
                              onRequestDelete={task => {
                                setDeleteError('');
                                setPendingDeleteTask(task);
                              }}
                            />
                          ))}
                          {completedTasks.length > 0 && (
                            <>
                              <button
                                type="button"
                                onClick={() => toggleCompletedSection(quadrant)}
                                aria-expanded={isExpanded}
                                className="flex items-center gap-1.5 px-2 py-1 text-[12px] font-medium text-slate-500 hover:text-slate-700 hover:bg-slate-100 rounded-md transition-colors w-fit"
                              >
                                {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                                <CheckCircle2 size={14} className="text-emerald-500" />
                                <span>已完成 ({completedTasks.length})</span>
                              </button>
                              {isExpanded && completedTasks.map(task => (
                                <TaskCard
                                  key={task.id}
                                  task={task}
                                  isClassifying={false}
                                  onOpenTask={task => onOpenTask({ ownerType: 'butler' }, task)}
                                  onToggleComplete={handleToggle}
                                  onRequestDelete={task => {
                                    setDeleteError('');
                                    setPendingDeleteTask(task);
                                  }}
                                />
                              ))}
                            </>
                          )}
                        </>
                      )}
                  </div>
                </section>
              );
            })}
          </div>
        )}
      </div>

      {pendingDeleteTask && (
        <Modal onClose={() => !isDeleting && setPendingDeleteTask(null)} width="w-[420px]" ariaLabel="确认删除任务">
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
