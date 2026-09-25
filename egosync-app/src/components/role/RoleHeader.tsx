import { useState, useRef, useEffect } from 'react';
import { ListTodo, BrainCircuit, Sliders, MoreHorizontal } from 'lucide-react';
import { cn } from '../../lib/utils';
import { getRoleIconComponent, normalizeColorHex } from '../../lib/roleIcons';
import type { Role } from '../../types/role';

export type RoleViewTab = 'tasks' | 'memory' | 'settings' | null;

interface RoleHeaderProps {
  role: Role;
  openTab: RoleViewTab;
  onToggleTab: (tab: Exclude<RoleViewTab, null>) => void;
  /** Story 16.4：移动端「⋯」菜单——切换角色（回角色列表根） */
  onSwitchRole?: () => void;
  /** Story 16.4：移动端「⋯」菜单——新建角色直达 */
  onAddRole?: () => void;
}

/**
 * 角色视图头部 — 从 RoleView 抽取出来的独立组件 (Story 2.2 AC-5)。
 *
 * 设计原则：
 * - icon/color 一律走 `lib/roleIcons.ts` 白名单，与 Sidebar 共用一套真相
 *   （Story 1.9/2.1 经验：mock 字段 `role.text`/Tailwind class 已废弃，不再兼容）
 * - active tab 的着色用 inline style 注入 `role.color`，不引入 `role.text` 这种
 *   只在 mock 模式下有效的 Tailwind class
 */
export function RoleHeader({ role, openTab, onToggleTab, onSwitchRole, onAddRole }: RoleHeaderProps) {
  const RoleIcon = getRoleIconComponent(role.icon);
  const roleColor = normalizeColorHex(role.color);
  // Story 16.4：移动端「⋯」菜单状态（切换角色/新建角色直达——线框屏 2b）
  const [showMoreMenu, setShowMoreMenu] = useState(false);
  const moreMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!showMoreMenu) return;
    const handleClick = (e: MouseEvent) => {
      if (moreMenuRef.current && !moreMenuRef.current.contains(e.target as Node)) {
        setShowMoreMenu(false);
      }
    };
    // 键盘退出（Escape）——与 ButlerSettingsContent/GlobalSettingsModal
    // 的 Escape 范式一致；role=menu 必须可键盘关闭（a11y）
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        setShowMoreMenu(false);
      }
    };
    document.addEventListener('mousedown', handleClick);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleClick);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [showMoreMenu]);

  const tabButton = (tab: Exclude<RoleViewTab, null>, Icon: typeof ListTodo, label: string) => {
    const isActive = openTab === tab;
    return (
      <button
        onClick={() => onToggleTab(tab)}
        className={cn(
          // Story 16.4 评审修复：小屏触控 ≥44px（AC2 红线——px-2.5 py-2 仅
          // 约 36px）；桌面零变化
          'flex items-center gap-1.5 px-2.5 md:px-3 py-2 max-md:min-h-[44px] rounded-lg text-[13px] font-medium transition-all',
          isActive ? 'bg-white dark:bg-slate-700 shadow-sm' : 'text-slate-500 dark:text-slate-400 hover:bg-white/50 dark:hover:bg-slate-700/60',
        )}
        style={isActive ? { color: roleColor } : undefined}
      >
        <Icon size={16} /> {label}
      </button>
    );
  };

  return (
    // Story 16.2 响应式基线：小屏 px-4 收缩边距、目标文案与能量条收缩、
    // tab 按钮收窄（375px 不溢出；断点方案归 UX 勘注定稿）；≥md 维持桌面原样。
    // Story 16.4：relative 供移动端「⋯」菜单绝对定位（桌面零影响）；
    // max-md:z-20——backdrop-blur 令 header 成为层叠上下文（z-auto 且
    // 先于聊天区挂载 ⇒ 下拉菜单被后挂载的聊天区盖住，WebDriver 实测
    // click intercepted）。小屏抬层即可，桌面无重叠面零影响。
    <header className="relative max-md:z-20 h-auto md:h-[76px] py-2 md:py-0 border-b border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 backdrop-blur-md flex flex-wrap items-center px-4 md:px-8 gap-2 md:gap-4 shrink-0 shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
      <div
        className="w-[46px] h-[46px] rounded-xl flex items-center justify-center text-white shadow-sm shrink-0"
        style={{ backgroundColor: roleColor }}
      >
        <RoleIcon size={24} strokeWidth={2} />
      </div>
      <div className="min-w-0 max-w-full">
        <h2 className="font-semibold text-lg leading-tight text-slate-800 dark:text-slate-100 truncate">
          {role.name}
        </h2>
        <div className="flex items-center gap-2 md:gap-3 mt-1.5">
          {role.goal.trim() && (
            <span className="hidden min-[480px]:inline max-w-[140px] md:max-w-[260px] truncate text-[12px] text-slate-500 dark:text-slate-400 font-medium">
              {role.goal}
            </span>
          )}
          <div className="w-[70px] min-[480px]:w-[120px] h-[6px] bg-slate-200/80 dark:bg-slate-700 rounded-full overflow-hidden shrink-0">
            <div
              className="h-full bg-emerald-500 rounded-full transition-all duration-1000"
              style={{ width: `${role.energy}%` }}
            />
          </div>
          <span className="text-[11px] text-slate-500 dark:text-slate-400 font-medium shrink-0">{role.energy}% 能量</span>
        </div>
      </div>

      <div className="ml-auto relative flex items-center gap-1.5 md:gap-2" ref={moreMenuRef}>
        {/* Story 16.4：「⋯」= 切换角色/新建角色直达（线框屏 2b）——md:hidden
            桌面无此控件；触控 ≥44×44。 */}
        {onSwitchRole && (
          <>
            <button
              onClick={() => setShowMoreMenu(v => !v)}
              aria-label="角色菜单"
              aria-expanded={showMoreMenu}
              data-testid="role-more-menu"
              className="md:hidden w-11 h-11 flex items-center justify-center rounded-lg text-slate-400 dark:text-slate-500 hover:bg-white/50 dark:hover:bg-slate-700/60 transition-colors"
            >
              <MoreHorizontal size={20} />
            </button>
            {showMoreMenu && (
              <div
                role="menu"
                aria-label="角色操作菜单"
                className="md:hidden absolute right-2 top-[calc(100%-2px)] z-30 w-36 rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-lg py-1 animate-in fade-in zoom-in-95 duration-150"
              >
                <button
                  role="menuitem"
                  onClick={() => { setShowMoreMenu(false); onSwitchRole(); }}
                  data-testid="role-menu-switch"
                  className="w-full text-left px-4 py-2.5 max-md:min-h-[44px] max-md:flex max-md:items-center text-[13px] font-medium text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700/60"
                >
                  切换角色
                </button>
                <button
                  role="menuitem"
                  onClick={() => { setShowMoreMenu(false); onAddRole?.(); }}
                  data-testid="role-menu-add"
                  className="w-full text-left px-4 py-2.5 max-md:min-h-[44px] max-md:flex max-md:items-center text-[13px] font-medium text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700/60"
                >
                  新建角色
                </button>
              </div>
            )}
          </>
        )}
        {tabButton('tasks', ListTodo, '任务')}
        {tabButton('memory', BrainCircuit, '记忆')}
        {tabButton('settings', Sliders, '设置')}
      </div>
    </header>
  );
}