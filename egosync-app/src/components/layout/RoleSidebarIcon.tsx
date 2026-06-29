import { cn } from '../../lib/utils';
import { getRoleIconComponent } from '../../lib/roleIcons';

interface RoleSidebarIconProps {
  role: {
    id: string;
    name: string;
    icon: string; // line-icon 标识符，如 'briefcase'、'dumbbell'
    color: string; // hex "#4F46E5" 或 Tailwind class "bg-indigo-600"
    energy: number;
  };
  isActive: boolean;
  onClick: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
}

function clampEnergy(value: number): number {
  if (typeof value !== 'number' || Number.isNaN(value)) return 0;
  return Math.max(0, Math.min(100, value));
}

function getEnergyColor(energy: number): string {
  if (energy >= 80) return '#10B981'; // 翠绿
  if (energy >= 40) return '#F59E0B'; // 琥珀
  return '#EF4444'; // 红色（与仪表盘 red-500 一致）
}

function renderIcon(icon: string) {
  // 统一渲染为黑白线框 Lucide 图标；未知 icon id 自动回退到 Target。
  const IconComponent = getRoleIconComponent(icon);
  return <IconComponent size={22} strokeWidth={2} />;
}

function getActiveStyle(color: string): React.CSSProperties {
  if (color.startsWith('#')) {
    return { backgroundColor: color };
  }
  return {};
}

function getActiveClassName(color: string): string {
  if (color.startsWith('#')) {
    return 'text-white shadow-lg scale-105';
  }
  // Tailwind class 格式 (mock 兼容)
  return `${color} text-white shadow-lg scale-105`;
}

export function RoleSidebarIcon({ role, isActive, onClick, onContextMenu }: RoleSidebarIconProps) {
  const energy = clampEnergy(role.energy);
  const energyColor = getEnergyColor(energy);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onClick();
    }
  };

  return (
    <button
      onClick={onClick}
      onContextMenu={onContextMenu}
      onKeyDown={handleKeyDown}
      title={role.name}
      aria-label={`${role.name} - 能量值 ${energy}%`}
      className={cn(
        "relative w-11 h-11 rounded-xl flex items-center justify-center transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500 focus-visible:ring-offset-2",
        isActive
          ? getActiveClassName(role.color)
          : "text-slate-500 hover:bg-slate-200 dark:hover:bg-slate-700",
        !isActive && "breathe"
      )}
      style={isActive ? getActiveStyle(role.color) : undefined}
    >
      {renderIcon(role.icon)}
      {/* 能量状态小点 */}
      <span
        className="absolute -top-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-[#F1F3F5] dark:border-slate-800 shadow-sm"
        style={{ backgroundColor: energyColor }}
        aria-hidden="true"
      />
    </button>
  );
}