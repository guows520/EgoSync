import { ListTodo, BrainCircuit, Sliders } from 'lucide-react';
import { cn } from '../../lib/utils';
import { getRoleIconComponent, normalizeColorHex } from '../../lib/roleIcons';
import type { Role } from '../../types/role';

export type RoleViewTab = 'tasks' | 'memory' | 'settings' | null;

interface RoleHeaderProps {
  role: Role;
  openTab: RoleViewTab;
  onToggleTab: (tab: Exclude<RoleViewTab, null>) => void;
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
export function RoleHeader({ role, openTab, onToggleTab }: RoleHeaderProps) {
  const RoleIcon = getRoleIconComponent(role.icon);
  const roleColor = normalizeColorHex(role.color);

  const tabButton = (tab: Exclude<RoleViewTab, null>, Icon: typeof ListTodo, label: string) => {
    const isActive = openTab === tab;
    return (
      <button
        onClick={() => onToggleTab(tab)}
        className={cn(
          'flex items-center gap-1.5 px-2.5 md:px-3 py-2 rounded-lg text-[13px] font-medium transition-all',
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
    <header className="h-auto md:h-[76px] py-2 md:py-0 border-b border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 backdrop-blur-md flex flex-wrap items-center px-4 md:px-8 gap-2 md:gap-4 shrink-0 shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
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

      <div className="ml-auto flex items-center gap-1.5 md:gap-2">
        {tabButton('tasks', ListTodo, '任务')}
        {tabButton('memory', BrainCircuit, '记忆')}
        {tabButton('settings', Sliders, '设置')}
      </div>
    </header>
  );
}