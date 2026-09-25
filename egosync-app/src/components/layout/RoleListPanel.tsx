import { ChevronRight, Home, Plus } from 'lucide-react';
import { cn } from '../../lib/utils';
import { getRoleIconComponent, normalizeColorHex } from '../../lib/roleIcons';
import type { Role } from '../../types/role';

// Story 16.4：移动端「角色」tab 第一级——角色列表（线框定稿
// spec-16-4-mobile-wireframe.html 屏 2a：管家固定首行 + 各角色行 + 右上
// 「＋」新建）。点选角色 = 切当前角色并进详情（第二级）；再点底部已选中
// 的「角色」tab 回本列表根（标准 tab 行为——App 驱动）。
// 桌面 ≥768px 不可达（BottomTabBar md:hidden），本组件仅移动形态渲染。
interface RoleListPanelProps {
  roles: Role[];
  /** 当前视图（'butler' 或角色 id）——驱动「当前」高亮。 */
  currentView: string;
  onSelectButler: () => void;
  onSelectRole: (roleId: string) => void;
  onAddRole: () => void;
}

export function RoleListPanel({ roles, currentView, onSelectButler, onSelectRole, onAddRole }: RoleListPanelProps) {
  const rowClass = (isActive: boolean) => cn(
    'w-full min-h-[44px] flex items-center gap-3 rounded-xl border px-4 py-3 text-left transition-colors motion-reduce:transition-none',
    isActive
      ? 'border-indigo-300 dark:border-indigo-600 bg-indigo-50/60 dark:bg-indigo-950/40'
      : 'border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800',
  );

  return (
    <div className="h-full flex flex-col bg-white/40 dark:bg-slate-900/40">
      {/* 头部：标题 + 右上「＋」新建（线框屏 2a） */}
      <header className="border-b border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 backdrop-blur-md flex items-center px-4 py-3 shrink-0 shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <h2 className="font-semibold text-lg leading-tight text-slate-800 dark:text-slate-100">角色</h2>
        <button
          type="button"
          onClick={onAddRole}
          aria-label="添加角色"
          data-testid="role-list-add"
          className="ml-auto w-11 h-11 flex items-center justify-center rounded-xl text-slate-400 hover:text-indigo-500 hover:bg-indigo-50/50 dark:hover:bg-indigo-900/30 transition-colors"
        >
          <Plus size={20} strokeWidth={2.5} />
        </button>
      </header>

      <div className="flex-1 overflow-y-auto p-4 space-y-2.5">
        {/* 管家固定首行（线框：🏠 数字分身管家） */}
        <button
          type="button"
          onClick={onSelectButler}
          data-testid="role-list-butler"
          className={rowClass(currentView === 'butler')}
        >
          <span className="w-10 h-10 rounded-xl bg-slate-800 dark:bg-indigo-600 text-white flex items-center justify-center shadow-sm shrink-0">
            <Home size={20} strokeWidth={2} />
          </span>
          <span className="flex-1 min-w-0">
            <span className="block text-[14px] font-semibold text-slate-800 dark:text-slate-100 truncate">数字分身管家</span>
            <span className="block text-[12px] text-slate-500 dark:text-slate-400 truncate">总管一切的个人助手</span>
          </span>
          {currentView === 'butler'
            ? <span className="shrink-0 text-[12px] font-semibold text-indigo-600 dark:text-indigo-400">当前</span>
            : <ChevronRight size={16} className="shrink-0 text-slate-300 dark:text-slate-600" />}
        </button>

        {roles.map(role => {
          const RoleIcon = getRoleIconComponent(role.icon);
          const roleColor = normalizeColorHex(role.color);
          const isActive = currentView === role.id;
          return (
            <button
              key={role.id}
              type="button"
              onClick={() => onSelectRole(role.id)}
              data-testid={`role-list-role-${role.id}`}
              className={rowClass(isActive)}
            >
              <span
                className="w-10 h-10 rounded-xl flex items-center justify-center text-white shadow-sm shrink-0"
                style={{ backgroundColor: roleColor }}
              >
                <RoleIcon size={20} strokeWidth={2} />
              </span>
              <span className="flex-1 min-w-0">
                <span className="block text-[14px] font-semibold text-slate-800 dark:text-slate-100 truncate">{role.name}</span>
                {role.goal.trim() && (
                  <span className="block text-[12px] text-slate-500 dark:text-slate-400 truncate">{role.goal}</span>
                )}
              </span>
              {isActive
                ? <span className="shrink-0 text-[12px] font-semibold text-indigo-600 dark:text-indigo-400">当前</span>
                : <ChevronRight size={16} className="shrink-0 text-slate-300 dark:text-slate-600" />}
            </button>
          );
        })}
      </div>
    </div>
  );
}
