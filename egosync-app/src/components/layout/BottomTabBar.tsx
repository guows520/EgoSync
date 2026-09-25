import { Home, Users, Settings as SettingsIcon } from 'lucide-react';
import { cn } from '../../lib/utils';

// Story 16.4：移动端底部 tab 导航（<768px 侧栏退场的替代形态）。
// 结构固定为 管家/角色/设置（UX 初案——数量/位置变更须产品负责人批准）；
// 桌面 ≥768px 经 md:hidden 整件退场（侧栏零变化）。
// 触控目标：每个 tab ≥44×44px（min-h + flex-1 全宽）；
// 安全区：pb-[max(1rem,env(safe-area-inset-bottom))]（照 ChatStream.tsx
// 输入区模式——iPhone 底部横条/手势条不压 tab；无 inset 设备维持 1rem）。
export type BottomTab = 'butler' | 'roles' | 'settings';

interface BottomTabBarProps {
  activeTab: BottomTab;
  onButlerTab: () => void;
  onRolesTab: () => void;
  onSettingsTab: () => void;
}

export function BottomTabBar({ activeTab, onButlerTab, onRolesTab, onSettingsTab }: BottomTabBarProps) {
  const tabs = [
    { key: 'butler', label: '管家', Icon: Home, onClick: onButlerTab },
    { key: 'roles', label: '角色', Icon: Users, onClick: onRolesTab },
    { key: 'settings', label: '设置', Icon: SettingsIcon, onClick: onSettingsTab },
  ] as const;

  return (
    <nav
      className="md:hidden shrink-0 border-t border-slate-200/60 dark:border-slate-700/60 bg-[#F1F3F5] dark:bg-slate-800 pb-[max(1rem,env(safe-area-inset-bottom))]"
      role="navigation"
      aria-label="主导航"
      data-testid="bottom-tab-bar"
    >
      <div className="flex items-stretch">
        {tabs.map(({ key, label, Icon, onClick }) => {
          const isActive = activeTab === key;
          return (
            <button
              key={key}
              type="button"
              onClick={onClick}
              aria-current={isActive ? 'page' : undefined}
              data-testid={`bottom-tab-${key}`}
              className={cn(
                'flex-1 min-h-[44px] flex flex-col items-center justify-center gap-0.5 pt-1.5 pb-1 text-[10px] font-medium transition-colors motion-reduce:transition-none',
                isActive
                  ? 'text-indigo-600 dark:text-indigo-400'
                  : 'text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300',
              )}
            >
              <Icon size={20} strokeWidth={isActive ? 2.5 : 2} />
              {label}
            </button>
          );
        })}
      </div>
    </nav>
  );
}
