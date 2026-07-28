import { useState, useRef, useMemo, useEffect } from 'react';
import { Play, Square, X, AtSign } from 'lucide-react';
import { cn } from '../../lib/utils';
import type { SelectableSkill } from '../../types/skill';

interface ChatInputProps {
  onSend: (content: string) => Promise<boolean>;
  onStop?: () => void;
  isStreaming?: boolean;
  disabled?: boolean;
  placeholder?: string;
  useRoleAccent?: boolean;
  availableSkills?: SelectableSkill[];
  selectedSkillKey?: string | null;
  onSelectedSkillChange?: (skillKey: string | null) => void;
}

export function ChatInput({
  onSend,
  onStop,
  isStreaming,
  disabled,
  placeholder,
  useRoleAccent,
  availableSkills = [],
  selectedSkillKey,
  onSelectedSkillChange,
}: ChatInputProps) {
  const [input, setInput] = useState('');
  const [internalSelectedSkillKey, setInternalSelectedSkillKey] = useState<string | null>(null);
  const effectiveSelectedSkillKey = selectedSkillKey !== undefined ? selectedSkillKey : internalSelectedSkillKey;
  const selectedSkill = availableSkills.find(skill => skill.key === effectiveSelectedSkillKey) ?? null;
  const [showSkillPicker, setShowSkillPicker] = useState(false);
  const [skillQuery, setSkillQuery] = useState('');
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const triggerStartRef = useRef<number | null>(null);

  const filteredSkills = useMemo(() => {
    if (!skillQuery) return availableSkills;
    const q = skillQuery.toLowerCase();
    return availableSkills.filter(s => s.name.toLowerCase().startsWith(q));
  }, [availableSkills, skillQuery]);

  useEffect(() => {
    setActiveIndex(filteredSkills.length > 0 ? 0 : -1);
  }, [showSkillPicker, filteredSkills.length]);

  const clearSkillSelection = () => {
    setInternalSelectedSkillKey(null);
    onSelectedSkillChange?.(null);
  };

  const selectSkill = (skill: SelectableSkill) => {
    setInternalSelectedSkillKey(skill.key);
    onSelectedSkillChange?.(skill.key);
    setShowSkillPicker(false);
    setSkillQuery('');
    // Remove the @trigger and query from input
    const triggerStart = triggerStartRef.current;
    if (triggerStart !== null) setInput(input.slice(0, triggerStart));
    triggerStartRef.current = null;
    inputRef.current?.focus();
  };

  const handleSend = async () => {
    const trimmed = input.trim();
    if (!trimmed || disabled) return;
    const accepted = await onSend(trimmed);
    if (accepted !== false) {
      setInput('');
      clearSkillSelection();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (showSkillPicker) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (filteredSkills.length > 0) setActiveIndex(prev => Math.min(prev + 1, filteredSkills.length - 1));
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (filteredSkills.length > 0) setActiveIndex(prev => Math.max(prev - 1, 0));
        return;
      }
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        if (filteredSkills[activeIndex]) {
          selectSkill(filteredSkills[activeIndex]);
          return;
        }
        setShowSkillPicker(false);
        setSkillQuery('');
        void handleSend();
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        setShowSkillPicker(false);
        setSkillQuery('');
        return;
      }
    } else {
      if (e.key === 'Enter' && !e.shiftKey) {
        void handleSend();
      }
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setInput(value);

    // Detect @ trigger: @ at end of input or @ followed by non-space chars
    const atMatch = value.match(/(?:^|\s)@(\S*)$/);
    if (atMatch) {
      triggerStartRef.current = value.length - atMatch[1].length - 1;
      setShowSkillPicker(true);
      setSkillQuery(atMatch[1]);
    } else if (showSkillPicker) {
      setShowSkillPicker(false);
      setSkillQuery('');
      triggerStartRef.current = null;
    }
  };

  const activeOptionId = showSkillPicker && filteredSkills[activeIndex]
    ? `skill-option-${activeIndex}`
    : undefined;

  return (
    <div className="relative w-full">
      {/* Selected skill chip */}
      {selectedSkill && (
        <div className="mb-1.5 flex items-center gap-1.5">
          <span className="inline-flex items-center gap-1 rounded-md bg-slate-100 dark:bg-slate-700 px-2 py-0.5 text-xs text-slate-700 dark:text-slate-200">
            <AtSign size={11} className="text-slate-400" />
            {selectedSkill.name}
            <button
              type="button"
              onClick={clearSkillSelection}
              aria-label="移除 Skill 选择"
              className="ml-0.5 hover:text-slate-900 dark:hover:text-white"
            >
              <X size={12} />
            </button>
          </span>
        </div>
      )}

      <div className="relative">
      <input
        ref={inputRef}
        type="text"
        value={input}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        placeholder={placeholder ?? "跟 管家 说点什么，比如：查看一下我有哪些任务…"}
        aria-label={placeholder || "跟管家说点什么"}
        role="combobox"
        aria-autocomplete="list"
        aria-expanded={showSkillPicker}
        aria-controls={showSkillPicker ? 'skill-listbox' : undefined}
        aria-activedescendant={activeOptionId}
        disabled={disabled}
        className="w-full bg-white dark:bg-slate-700 border border-slate-200 dark:border-slate-600 rounded-xl pl-5 pr-14 py-3.5 text-[14px] dark:text-slate-100 dark:placeholder:text-slate-400 focus:outline-none focus:border-transparent focus:ring-1 focus:ring-slate-500/20 focus-visible:!outline-none transition-colors shadow-sm disabled:opacity-60 disabled:cursor-not-allowed"
      />

      {/* Skill picker dropdown */}
      {showSkillPicker && (
        <div
          id="skill-listbox"
          ref={listRef}
          role="listbox"
          aria-label="可用 Skill 列表"
          className="absolute bottom-full left-0 mb-1 w-full max-h-48 overflow-y-auto bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-600 rounded-lg shadow-lg z-10"
        >
          {filteredSkills.length === 0 ? (
            <div className="px-4 py-3 text-sm text-slate-400 dark:text-slate-500">
              {availableSkills.length === 0
                ? '当前没有可用的 Skill'
                : '没有匹配的 Skill'}
            </div>
          ) : (
            filteredSkills.map((skill, index) => (
              <div
                key={skill.key}
                id={`skill-option-${index}`}
                role="option"
                aria-selected={index === activeIndex}
                onMouseDown={(e) => {
                  e.preventDefault();
                  selectSkill(skill);
                }}
                onMouseEnter={() => setActiveIndex(index)}
                className={cn(
                  'px-4 py-2 cursor-pointer text-sm transition-colors',
                  index === activeIndex
                    ? 'bg-slate-100 dark:bg-slate-700'
                    : 'hover:bg-slate-50 dark:hover:bg-slate-750',
                )}
              >
                <div className="font-medium text-slate-800 dark:text-slate-200">{skill.name}</div>
                {skill.description && (
                  <div className="text-xs text-slate-400 dark:text-slate-500 truncate">{skill.description}</div>
                )}
              </div>
            ))
          )}
        </div>
      )}

      {isStreaming ? (
        <button
          onClick={onStop}
          aria-label="停止"
          className="absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center text-white bg-slate-800 rounded-lg transition-colors shadow-sm hover:bg-slate-700"
        >
          <Square size={14} fill="currentColor" />
        </button>
      ) : (
        <button
          onClick={() => { void handleSend(); }}
          disabled={disabled || !input.trim()}
          aria-label="发送"
          className={cn(
            'absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center text-white rounded-lg transition-colors shadow-sm disabled:opacity-50 disabled:cursor-not-allowed',
            useRoleAccent ? 'hover:brightness-110' : 'bg-slate-800 hover:bg-slate-700',
          )}
          style={useRoleAccent ? { backgroundColor: 'var(--role-accent)' } : undefined}
        >
          <Play size={16} className="ml-0.5" fill="currentColor" />
        </button>
      )}
      </div>
    </div>
  );
}
