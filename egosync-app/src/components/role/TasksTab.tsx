import { useState, type ReactNode } from 'react';
import { AlertTriangle, CheckCircle2, ChevronDown, ChevronRight, Circle, Edit2, Filter, GripVertical, Loader2, Plus, Target, Trash2 } from 'lucide-react';
import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  pointerWithin,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  arrayMove,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Modal } from '../layout/Modal';
import { cn, formatDeadline } from '../../lib/utils';
import { type Task, type TaskQuadrant } from '../../types/task';

interface TasksTabProps {
  role: { color?: string };
  tasks: Task[];
  isLoading: boolean;
  error: string | null;
  /** 后台正在异步智能分类的任务 id，在卡片上显示「分类中」过渡态。缺省视为无分类中任务。 */
  classifyingIds?: Set<string>;
  onOpenTask: (task: Task | null) => void;
  onDeleteTask: (id: string) => Promise<void> | void;
  onReorderTasks: (taskIds: string[]) => Promise<void> | void;
  onToggleComplete: (id: string, isCompleted: boolean) => Promise<void> | void;
}

interface TaskCardCallbacks {
  onOpenTask: (task: Task) => void;
  onToggle: (task: Task) => void;
  onRequestDelete: (task: Task) => void;
}

const EMPTY_CLASSIFYING_IDS: Set<string> = new Set();

const CARD_BASE_CLASS =
  'bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-4 flex gap-3.5 shadow-sm group hover:border-indigo-300 dark:hover:border-indigo-600 hover:shadow-md transition-all duration-200 motion-reduce:transition-none';

function TaskCardBody({ task, callbacks, dragHandle, isClassifying }: { task: Task; callbacks: TaskCardCallbacks; dragHandle?: ReactNode; isClassifying?: boolean }) {
  return (
    <>
      {dragHandle}
      <button
        onClick={event => {
          event.stopPropagation();
          callbacks.onToggle(task);
        }}
        className={cn(
          'transition-colors shrink-0 mt-0.5',
          task.isCompleted ? 'text-emerald-500' : 'text-slate-300 dark:text-slate-600 hover:text-emerald-500',
        )}
        aria-label={`${task.isCompleted ? '撤销完成' : '完成'} ${task.title}`}
      >
        {task.isCompleted ? <CheckCircle2 size={20} strokeWidth={2.5} /> : <Circle size={20} strokeWidth={2.5} />}
      </button>
      <div className="flex-1 min-w-0">
        <p
          className={cn(
            'text-[14.5px] font-medium leading-snug',
            task.isCompleted ? 'text-slate-400 dark:text-slate-500 line-through' : 'text-slate-800 dark:text-slate-100',
          )}
        >
          {task.title}
        </p>
        <div className="flex items-center gap-2 mt-2.5 flex-wrap">
          {task.deadline && <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-transparent text-slate-500 dark:text-slate-400 border border-slate-300 dark:border-slate-600">{formatDeadline(task.deadline)}</span>}
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
      <div className="flex items-start gap-1 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity">
        <button
          onClick={event => {
            event.stopPropagation();
            callbacks.onOpenTask(task);
          }}
          className="p-1.5 rounded-md text-slate-400 dark:text-slate-500 hover:text-indigo-600 hover:bg-indigo-50 dark:hover:bg-indigo-900/30"
          aria-label={`编辑 ${task.title}`}
        >
          <Edit2 size={15} />
        </button>
        <button
          onClick={event => {
            event.stopPropagation();
            callbacks.onRequestDelete(task);
          }}
          className="p-1.5 rounded-md text-slate-400 dark:text-slate-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/30"
          aria-label={`删除 ${task.title}`}
        >
          <Trash2 size={15} />
        </button>
      </div>
    </>
  );
}

function pointerWithinFallbackToClosestCenter(args: Parameters<typeof pointerWithin>[0]) {
  const pointerCollisions = pointerWithin(args);
  return pointerCollisions.length > 0 ? pointerCollisions : closestCenter(args);
}

function SortableTaskCard({ task, callbacks, isClassifying, disabled = false }: { task: Task; callbacks: TaskCardCallbacks; isClassifying?: boolean; disabled?: boolean }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: task.id, disabled });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition: transition ?? 'transform 200ms ease',
  };
  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(CARD_BASE_CLASS, isDragging && 'opacity-50', task.protectionStatus === 'at_risk' && 'border-l-4 border-l-amber-400')}
    >
      <TaskCardBody
        task={task}
        callbacks={callbacks}
        isClassifying={isClassifying}
        dragHandle={
          disabled ? undefined : (
            <button
              {...attributes}
              {...listeners}
              onClick={event => event.stopPropagation()}
              className="text-slate-300 dark:text-slate-600 hover:text-slate-500 cursor-grab active:cursor-grabbing shrink-0 mt-0.5 touch-none"
              aria-label={`拖动排序 ${task.title}`}
            >
              <GripVertical size={18} />
            </button>
          )
        }
      />
    </div>
  );
}

function CompletedTaskCard({ task, callbacks }: { task: Task; callbacks: TaskCardCallbacks }) {
  return (
    <div
      className={cn(CARD_BASE_CLASS, 'opacity-60 scale-[0.99]')}
    >
      <TaskCardBody task={task} callbacks={callbacks} />
    </div>
  );
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

const quadrantChips: Array<{ value: TaskQuadrant | 'all'; label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'Q1', label: 'Q1' },
  { value: 'Q2', label: 'Q2' },
  { value: 'Q3', label: 'Q3' },
  { value: 'Q4', label: 'Q4' },
];

export function TasksTab({ role, tasks, isLoading, error, classifyingIds = EMPTY_CLASSIFYING_IDS, onOpenTask, onDeleteTask, onReorderTasks, onToggleComplete }: TasksTabProps) {
  const [pendingDeleteTask, setPendingDeleteTask] = useState<Task | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState('');
  const [activeId, setActiveId] = useState<string | null>(null);
  const [actionError, setActionError] = useState('');
  const [quadrantFilter, setQuadrantFilter] = useState<TaskQuadrant | 'all'>('all');
  const [showBigRocksOnly, setShowBigRocksOnly] = useState(false);
  // 按象限折叠已完成任务列表：默认折叠，避免占据大量空间淫没未完成任务
  const [expandedCompleted, setExpandedCompleted] = useState<Record<TaskQuadrant, boolean>>({
    Q1: false,
    Q2: false,
    Q3: false,
    Q4: false,
  });
  const toggleCompletedSection = (quadrant: TaskQuadrant) => {
    setExpandedCompleted(prev => ({ ...prev, [quadrant]: !prev[quadrant] }));
  };
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

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
    return <div className="text-[13px] text-slate-500 dark:text-slate-400">任务加载中...</div>;
  }

  if (error) {
    return <div className="text-[13px] text-red-600">{error}</div>;
  }

  const activeStyle = role.color ? { color: role.color, borderColor: role.color } : undefined;
  const allQuadrants = Object.keys(quadrantLabels) as TaskQuadrant[];
  const visibleTasks = tasks.filter(task => (
    (quadrantFilter === 'all' || task.quadrant === quadrantFilter)
    && (!showBigRocksOnly || task.isBigRock)
  ));
  const groupedTasks = visibleTasks.reduce<Record<TaskQuadrant, Task[]>>(
    (groups, task) => {
      groups[task.quadrant].push(task);
      return groups;
    },
    { Q1: [], Q2: [], Q3: [], Q4: [] },
  );

  const orderBySort = (a: Task, b: Task) => (Number(b.isBigRock) - Number(a.isBigRock)) || (a.sortOrder - b.sortOrder);
  const isFiltered = quadrantFilter !== 'all' || showBigRocksOnly;
  const visibleQuadrants = quadrantFilter === 'all' ? allQuadrants : [quadrantFilter];
  const incompleteByQuadrant = {} as Record<TaskQuadrant, Task[]>;
  const completedByQuadrant = {} as Record<TaskQuadrant, Task[]>;
  allQuadrants.forEach(quadrant => {
    const group = groupedTasks[quadrant];
    incompleteByQuadrant[quadrant] = group.filter(task => !task.isCompleted).sort(orderBySort);
    completedByQuadrant[quadrant] = group.filter(task => task.isCompleted).sort(orderBySort);
  });

  const callbacks: TaskCardCallbacks = {
    onOpenTask: task => onOpenTask(task),
    onToggle: task => {
      setActionError('');
      void Promise.resolve(onToggleComplete(task.id, !task.isCompleted)).catch(e => {
        console.error('切换任务完成状态失败:', e);
        setActionError('任务状态暂时切换失败，请稍后再试');
      });
    },
    onRequestDelete: task => {
      setDeleteError('');
      setPendingDeleteTask(task);
    },
  };

  const activeTask = activeId ? tasks.find(task => task.id === activeId) ?? null : null;

  const handleDragStart = (event: DragStartEvent) => {
    setActiveId(String(event.active.id));
  };

  const handleDragEnd = (event: DragEndEvent) => {
    setActiveId(null);
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    if (isFiltered) return;
    const activeKey = String(active.id);
    const overKey = String(over.id);
    const quadrant = allQuadrants.find(q => incompleteByQuadrant[q].some(task => task.id === activeKey));
    if (!quadrant) return;
    const list = incompleteByQuadrant[quadrant];
    const oldIndex = list.findIndex(task => task.id === activeKey);
    const newIndex = list.findIndex(task => task.id === overKey);
    if (oldIndex === -1 || newIndex === -1) return;
    const reordered = arrayMove(list, oldIndex, newIndex);
    const fullOrder: string[] = [];
    allQuadrants.forEach(q => {
      const incomplete = q === quadrant ? reordered : incompleteByQuadrant[q];
      incomplete.forEach(task => fullOrder.push(task.id));
      completedByQuadrant[q].forEach(task => fullOrder.push(task.id));
    });
    setActionError('');
    void Promise.resolve(onReorderTasks(fullOrder)).catch(e => {
      console.error('任务排序失败:', e);
      setActionError('任务排序暂时保存失败，请稍后再试');
    });
  };

  return (
    <>
      <div className="space-y-6">
        <div className="flex flex-col gap-3">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h3 className="text-[15px] font-semibold text-slate-800 dark:text-slate-100">任务清单</h3>
              <p className="mt-0.5 text-[12px] text-slate-500 dark:text-slate-400">管理当前角色的任务</p>
            </div>
            <button
              onClick={() => onOpenTask(null)}
              className="inline-flex items-center gap-1.5 hover:bg-slate-200 px-2.5 py-1.5 rounded-md transition-colors text-[13px] font-medium w-fit shrink-0"
              style={activeStyle}
            >
              <Plus size={16}/> 新增任务
            </button>
          </div>

          <div className="flex items-center gap-1 flex-nowrap rounded-xl border border-slate-200 dark:border-slate-700 bg-white/70 dark:bg-slate-800/70 px-2.5 py-2" role="group" aria-label="按象限筛选">
            <Filter size={14} className="text-slate-400 dark:text-slate-500 shrink-0" />
            {quadrantChips.map(chip => (
              <button
                key={chip.value}
                type="button"
                onClick={() => setQuadrantFilter(chip.value)}
                aria-pressed={quadrantFilter === chip.value}
                className={cn(
                  'px-2 py-0.5 rounded-md text-[12px] font-medium transition-colors motion-reduce:transition-none whitespace-nowrap',
                  quadrantFilter === chip.value ? 'text-slate-700 dark:text-slate-300' : 'text-slate-500 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-700',
                )}
                style={quadrantFilter === chip.value ? { backgroundColor: 'color-mix(in srgb, var(--role-accent) 15%, white)', color: 'var(--role-accent)' } : undefined}
              >
                {chip.label}
              </button>
            ))}
            <span className="w-px h-4 bg-slate-200 mx-0.5 shrink-0" />
            <button
              type="button"
              onClick={() => setShowBigRocksOnly(prev => !prev)}
              aria-pressed={showBigRocksOnly}
              className={cn(
                'inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[12px] font-medium transition-colors motion-reduce:transition-none whitespace-nowrap',
                showBigRocksOnly ? 'bg-amber-100 text-amber-700' : 'text-slate-500 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-700',
              )}
            >
              <Target size={13} /> 只看大石头
            </button>
          </div>
        </div>

      {actionError && (
        <div role="alert" className="text-[13px] text-red-600">{actionError}</div>
      )}

      {visibleTasks.length === 0 ? (
        <div className="rounded-xl border border-dashed border-slate-200 dark:border-slate-700 bg-white/70 dark:bg-slate-800/70 p-6 text-center text-[13px] text-slate-500 dark:text-slate-400">
          {tasks.length === 0 ? '还没有任务，先添加一个小目标吧' : '当前筛选无匹配任务'}
        </div>
      ) : (
        <DndContext
          sensors={sensors}
          collisionDetection={pointerWithinFallbackToClosestCenter}
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
          onDragCancel={() => setActiveId(null)}
        >
          {visibleQuadrants.map(quadrant => {
            const count = groupedTasks[quadrant].length;
            const completedTasks = completedByQuadrant[quadrant];
            const isExpanded = expandedCompleted[quadrant];
            const titleSpan = (
              <span className={cn('text-[12px] font-bold tracking-widest', quadrantTitleColor[quadrant])}>
                {quadrantLabels[quadrant]}
              </span>
            );
            const countBadge = (
              <span className="inline-flex items-center justify-center min-w-[20px] px-1.5 rounded-full bg-slate-100 dark:bg-slate-700 text-slate-500 dark:text-slate-400 text-[11px] font-semibold">
                {count}
              </span>
            );

            if (count === 0) {
              return (
                <div key={quadrant}>
                  <div className="flex items-center justify-between mb-3">
                    <span className="flex items-center gap-1.5">
                      <span className="w-3.5 shrink-0" aria-hidden="true" />
                      {titleSpan}
                    </span>
                    {countBadge}
                  </div>
                  <p className="text-[12.5px] text-slate-400 dark:text-slate-500">{quadrantEmptyHint[quadrant]}</p>
                </div>
              );
            }

            return (
              <div key={quadrant}>
                <div className="flex items-center justify-between w-full mb-3">
                  <span className="flex items-center gap-1.5">
                    {titleSpan}
                  </span>
                  {countBadge}
                </div>
                <div className="space-y-2.5">
                  <SortableContext
                    items={incompleteByQuadrant[quadrant].map(task => task.id)}
                    strategy={verticalListSortingStrategy}
                  >
                    {incompleteByQuadrant[quadrant].map(task => (
                      <SortableTaskCard key={task.id} task={task} callbacks={callbacks} isClassifying={classifyingIds.has(task.id)} disabled={isFiltered} />
                    ))}
                  </SortableContext>
                  {completedTasks.length > 0 && (
                    <>
                      <button
                        type="button"
                        onClick={() => toggleCompletedSection(quadrant)}
                        aria-expanded={isExpanded}
                        className="flex items-center gap-1.5 px-2 py-1 text-[12px] font-medium text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-md transition-colors w-fit"
                      >
                        {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                        <CheckCircle2 size={14} className="text-emerald-500" />
                        <span>已完成 ({completedTasks.length})</span>
                      </button>
                      {isExpanded && completedTasks.map(task => (
                        <CompletedTaskCard key={task.id} task={task} callbacks={callbacks} />
                      ))}
                    </>
                  )}
                </div>
              </div>
            );
          })}
          <DragOverlay dropAnimation={{ duration: 200, easing: 'cubic-bezier(0.18, 0.67, 0.6, 1.22)' }}>
            {activeTask ? (
              <div className={cn(CARD_BASE_CLASS, 'shadow-lg opacity-90', activeTask.protectionStatus === 'at_risk' && 'border-l-4 border-l-amber-400')}>
                <TaskCardBody task={activeTask} callbacks={callbacks} />
              </div>
            ) : null}
          </DragOverlay>
        </DndContext>
      )}
      </div>
      {pendingDeleteTask && (
        <Modal onClose={() => !isDeleting && setPendingDeleteTask(null)} width="w-[420px]" ariaLabel="确认删除任务">
          <div className="p-6">
            <h2 className="text-[18px] font-semibold text-slate-800 dark:text-slate-100">确认删除任务？</h2>
            <p className="mt-3 text-[13px] leading-6 text-slate-500 dark:text-slate-400">
              删除后任务会从当前清单移除，但历史记录会保留。确定要删除“{pendingDeleteTask.title}”吗？
            </p>
            {deleteError && <p className="mt-4 text-[12px] text-red-600">{deleteError}</p>}
            <div className="mt-6 flex justify-end gap-3">
              <button
                onClick={() => setPendingDeleteTask(null)}
                disabled={isDeleting}
                className="px-4 py-2 border border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-300 rounded-lg text-[13px] font-medium hover:bg-slate-50 dark:hover:bg-slate-700 disabled:opacity-60"
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
