import { useState, useEffect } from 'react';
import { cn, isMobileViewport } from '../../lib/utils';
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
  onArchiveRole,
  onDeleteRole,
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
      // 小屏 tab 全屏化（16.4 布局修复）：对话区 max-md:hidden 时来源跳转
      // 无可见效果——先关工作区 tab 回对话，ChatStream 再滚动定位；桌面
      // 双栏下对话区常驻可见，不关 tab，保持既有行为零变化
      if (isMobileViewport()) setOpenTab(null);
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
      <RoleHeader role={role} openTab={openTab} onToggleTab={toggleTab} onSwitchRole={onSwitchRole} onAddRole={onAddRole} onArchiveRole={onArchiveRole} onDeleteRole={onDeleteRole} />

      {/* Story 16.2 响应式基线：≥md 维持 65/35 双栏；小屏 flex-col。
          16.4 布局修复（2026-09-26 人类指令）：tab 打开时小屏对话区
          max-md:hidden、工作区 max-md:flex-1 全屏——点「任务/记忆/设置」
          只显示对应页面（drill-down 换页，替代原 58/42 堆叠）；再点一次
          当前 tab 按钮关闭回对话（既有 toggle 语义）。桌面双栏零变化；
          原 G11 inert h-[42%] 垫片随 58/42 布局一并移除（既有钉孔已按
          新布局改写，owner 授权解冻）。 */}
      <div className="flex-1 flex flex-col md:flex-row overflow-hidden">
        <div
          className={cn(
            'flex flex-col relative bg-white/40 dark:bg-slate-900/40 transition-all duration-500 ease-in-out min-h-0',
            openTab
              ? 'w-full max-md:hidden md:h-auto md:w-[65%] border-b md:border-b-0 md:border-r border-slate-200/60 dark:border-slate-700/60'
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
              ? 'w-full max-md:flex-1 md:h-auto md:w-[35%] border-t md:border-t-0 md:border-l opacity-100'
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