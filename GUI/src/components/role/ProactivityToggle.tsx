import { cn } from '../../lib/utils';
import type { ProactivityLevel } from '../../types/role';

interface ProactivityToggleProps {
  level: ProactivityLevel;
  onChange: (level: ProactivityLevel) => void;
  disabled?: boolean;
}

const OPTIONS: Array<{ value: ProactivityLevel; label: string }> = [
  { value: 'passive', label: '静默执行' },
  { value: 'moderate', label: '适度建议' },
  { value: 'proactive', label: '积极主动' },
];

export function ProactivityToggle({ level, onChange, disabled = false }: ProactivityToggleProps) {
  return (
    <div className="bg-slate-100/80 rounded-xl p-1.5 flex shadow-inner">
      {OPTIONS.map(option => (
        <button
          key={option.value}
          type="button"
          disabled={disabled}
          onClick={() => onChange(option.value)}
          className={cn(
            'flex-1 py-2 text-[13px] font-medium rounded-lg transition-colors disabled:cursor-not-allowed disabled:opacity-60',
            level === option.value
              ? 'bg-white shadow-sm text-indigo-600 border border-slate-200/50'
              : 'text-slate-500 hover:text-slate-800',
          )}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
