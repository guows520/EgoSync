import { useState, useEffect } from 'react';
import { cn } from '../../lib/utils';
import { RoleHeader, type RoleViewTab } from './RoleHeader';
import { RoleWorkspacePanel } from './RoleWorkspacePanel';
import { ChatStream } from '../chat/ChatStream';
import type { Role } from '../../types/role';
import type { SourceNavigationTarget } from '../../types/chat';
import type { Task, TaskActions } from '../../types/task';
import type { TaskScope } from '../../hooks/useTasks';

interface RoleViewProps {
  role: Role;
  roles?: Role[];
  onOpenTask: (scope: TaskScope, task?: Task | null) => void;
  onTasksApiReady?: (key: string, actions: TaskActions) => void;
  initialTab: string | null;
  onTabConsumed: () => void;
  onUpdateRole: (role: Role) => void;
  onArchiveRole?: (id: string) => Promise<void> | void;
  onDeleteRole?: (id: string) => Promise<void> | void;
  activeRoleCount?: number;
  sourceNavigationTarget?: SourceNavigationTarget | null;
  onSourceNavigationHandled?: () => void;
  onButlerSourceNavigation?: (target: SourceNavigationTarget) => void;
  onRoleSourceNavigation?: (target: SourceNavigationTarget) => void;
  /** Story 16.4：移动端角色详情头部「⋯」——切换角色（回角色列表根） */
  onSwitchRole?: () => void;
  /** Story 16.4：移动端角色详情头部「⋯」——新建角色直达 */
  onAddRole?: () => void;
}

export function RoleView({
  role,
  roles,
  onOpenTask,
  onTasksApiReady,
  initialTab,
  onTabConsumed,
  onUpdateRole,
  sourceNavigationTarget: externalSourceNavigationTarget,
  onSourceNavigationHandled,
  onButlerSourceNavigation,
  onRoleSourceNavigation,
  onSwitchRole,
  onAddRole,
}: RoleViewProps) {
  const [openTab, setOpenTab] = useState<RoleViewTab>(
    (initialTab as RoleViewTab) ?? null,
  );
  const [targetMemoryId, setTargetMemoryId] = useState<string | null>(null);
  const [sourceNavigationTarget, setSourceNavigationTarget] = useState<SourceNavigationTarget | null>(null);

  useEffect(() => {
    if (initialTab && openTab !== initialTab) {
      setOpenTab(initialTab as RoleViewTab);
      onTabConsumed?.();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialTab]);

  useEffect(() => {
    if (!externalSourceNavigationTarget) return;
    setSourceNavigationTarget(externalSourceNavigationTarget);
  }, [externalSourceNavigationTarget]);

  const toggleTab = (tab: Exclude<RoleViewTab, null>) => {
    setOpenTab(prev => (prev === tab ? null : tab));
  };

  const handleMemoryReferenceClick = (memoryId: string) => {
    setTargetMemoryId(memoryId);
    setOpenTab('memory');
  };

  const handleSourceMessageClick = (target: SourceNavigationTarget) => {
    if (target.roleId === role.id) {
      setSourceNavigationTarget(target);
      return;
    }
    if (target.roleId === null) {
      onButlerSourceNavigation?.(target);
      return;
    }
    onRoleSourceNavigation?.(target);
  };

  return (
    <div className="h-full flex flex-col transition-colors duration-300 animate-in fade-in duration-300">
      <RoleHeader role={role} openTab={openTab} onToggleTab={toggleTab} onSwitchRole={onSwitchRole} onAddRole={onAddRole} />

      {/* Story 16.2 响应式基线：≥md 维持 65/35 双栏；小屏（375px 级）
          flex-col 堆叠——对话区 58% + 工作区 42%（断点方案归 UX 勘注定稿）。 */}
      {/* Story 16.4 修复注记：小屏比例高由百分比 h-[58%]/h-[42%] 改为
          max-md:flex-[58]/[42]——百分比高对 flex 列子项解析退化（实测
          工作区塌缩至 53px、内容溢出外壳，排序/调序按钮不可达）；flex
          比例对定高容器免疫。桌面 md:h-auto/md:w-* 零变化。
          [评审轮 G11] 工作区侧保留 h-[42%] 为 inert 遗留垫片：本行是
          flex 列子项，flex-basis（max-md:flex-[42] 的 flex:42 1 0%）
          在主轴上优先于 height，h-[42%] 不参与实际布局；保留它只为
          让 16.2 既有测试（RoleView.test.tsx 的 h-[42%] 钉孔）原样
          通过——冻结块「禁改既有断言」，改组件比改断言优先。ButlerView
          无此既有钉孔，故不需要垫片。 */}
      <div className="flex-1 flex flex-col md:flex-row overflow-hidden">
        <div
          className={cn(
            'flex flex-col relative bg-white/40 dark:bg-slate-900/40 transition-all duration-500 ease-in-out min-h-0',
            openTab
              ? 'w-full max-md:flex-[58] md:h-auto md:w-[65%] border-b md:border-b-0 md:border-r border-slate-200/60 dark:border-slate-700/60'
              : 'w-full h-full',
          )}
        >
          <ChatStream
            role={role}
            onMemoryReferenceClick={handleMemoryReferenceClick}
            sourceNavigationTarget={sourceNavigationTarget}
            onSourceNavigationHandled={() => {
              setSourceNavigationTarget(null);
              onSourceNavigationHandled?.();
            }}
          />
        </div>

        <div
          className={cn(
            'bg-white dark:bg-slate-900 flex flex-col border-slate-200/60 dark:border-slate-700/60 overflow-hidden transition-all duration-500 ease-in-out min-h-0',
            openTab
              ? 'w-full h-[42%] max-md:flex-[42] md:h-auto md:w-[35%] border-t md:border-t-0 md:border-l opacity-100'
              : 'w-0 h-0 md:h-auto opacity-0 border-0 shadow-none pointer-events-none',
          )}
          aria-hidden={!openTab}
        >
          {openTab && (
            <RoleWorkspacePanel
              role={role}
              roles={roles}
              currentTab={openTab}
              setTab={setOpenTab}
              onOpenTask={(task: Task | null) => onOpenTask({ ownerType: 'role', roleId: role.id }, task)}
              onTasksApiReady={(actions: TaskActions) => onTasksApiReady?.(`role:${role.id}`, actions)}
              onUpdateRole={onUpdateRole}
              targetMemoryId={targetMemoryId}
              onTargetMemoryHandled={() => setTargetMemoryId(null)}
              onSourceMessageClick={handleSourceMessageClick}
            />
          )}
        </div>
      </div>
    </div>
  );
}