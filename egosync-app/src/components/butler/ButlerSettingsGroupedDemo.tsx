import { useEffect, useMemo, useState } from 'react';
import {
  Archive,
  Check,
  CheckCircle2,
  ChevronDown,
  Clock3,
  Info,
  Plus,
  Save,
  Sparkles,
  Target,
  User,
  Wand2,
} from 'lucide-react';
import { cn } from '../../lib/utils';

type SectionId = 'basic' | 'mission' | 'skills' | 'schedule' | 'archived';
type SkillSectionId = 'builtIn' | 'opencode' | 'custom';

const STORAGE_KEY = 'egosync-butler-settings-demo-sections-v1';

const SECTION_IDS: SectionId[] = ['basic', 'mission', 'skills', 'schedule', 'archived'];

const DEFAULT_OPEN_SECTIONS: Record<SectionId, boolean> = {
  basic: true,
  mission: true,
  skills: true,
  schedule: true,
  archived: false,
};

const DEFAULT_OPEN_SKILL_SECTIONS: Record<SkillSectionId, boolean> = {
  builtIn: true,
  opencode: false,
  custom: true,
};

interface SettingsSectionProps {
  id: string;
  title: string;
  description: string;
  summary?: string;
  icon: React.ReactNode;
  isOpen: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}

function SettingsSection({
  id,
  title,
  description,
  summary,
  icon,
  isOpen,
  onToggle,
  children,
}: SettingsSectionProps) {
  return (
    <section className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white shadow-sm dark:border-slate-700 dark:bg-slate-900">
      <button
        type="button"
        aria-expanded={isOpen}
        aria-controls={`settings-section-${id}`}
        onClick={onToggle}
        className="flex w-full items-center gap-3 px-5 py-4 text-left transition-colors hover:bg-slate-50 dark:hover:bg-slate-800/80"
      >
        <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-indigo-50 text-indigo-600 dark:bg-indigo-900/30 dark:text-indigo-300">
          {icon}
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex flex-wrap items-center gap-x-2 gap-y-1">
            <span className="text-[15px] font-semibold text-slate-800 dark:text-slate-100">{title}</span>
            {summary && (
              <span className="rounded-full border border-slate-200 bg-slate-50 px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-400">
                {summary}
              </span>
            )}
          </span>
          <span className="mt-1 block text-[12.5px] leading-relaxed text-slate-500 dark:text-slate-400">{description}</span>
        </span>
        <ChevronDown
          size={18}
          aria-hidden="true"
          className={cn('shrink-0 text-slate-400 transition-transform duration-200', !isOpen && '-rotate-90')}
        />
      </button>
      {isOpen && (
        <div id={`settings-section-${id}`} className="border-t border-slate-100 px-5 py-5 dark:border-slate-800">
          {children}
        </div>
      )}
    </section>
  );
}

interface SkillGroupProps {
  id: SkillSectionId;
  title: string;
  summary: string;
  isOpen: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}

function SkillGroup({ id, title, summary, isOpen, onToggle, children }: SkillGroupProps) {
  return (
    <div className="overflow-hidden rounded-xl border border-slate-200 dark:border-slate-700">
      <button
        type="button"
        aria-expanded={isOpen}
        aria-controls={`skill-section-${id}`}
        onClick={onToggle}
        className="flex w-full items-center justify-between gap-3 bg-slate-50 px-4 py-3 text-left transition-colors hover:bg-slate-100 dark:bg-slate-800/80 dark:hover:bg-slate-800"
      >
        <span className="min-w-0">
          <span className="block text-[13.5px] font-semibold text-slate-700 dark:text-slate-200">{title}</span>
          <span className="mt-0.5 block text-[11.5px] text-slate-400 dark:text-slate-500">{summary}</span>
        </span>
        <ChevronDown
          size={16}
          aria-hidden="true"
          className={cn('shrink-0 text-slate-400 transition-transform duration-200', !isOpen && '-rotate-90')}
        />
      </button>
      {isOpen && (
        <div id={`skill-section-${id}`} className="space-y-2 border-t border-slate-200 p-3 dark:border-slate-700">
          {children}
        </div>
      )}
    </div>
  );
}

function readSectionState<T extends string>(key: string, fallback: Record<T, boolean>): Record<T, boolean> {
  try {
    const stored = localStorage.getItem(key);
    if (!stored) return fallback;
    const parsed = JSON.parse(stored) as Partial<Record<T, boolean>>;
    return Object.fromEntries(
      Object.keys(fallback).map(id => [id, typeof parsed[id as T] === 'boolean' ? parsed[id as T] : fallback[id as T]])
    ) as Record<T, boolean>;
  } catch {
    return fallback;
  }
}

function ToggleRow({ title, description, enabled, onToggle }: { title: string; description: string; enabled: boolean; onToggle: () => void }) {
  return (
    <div className="flex items-start justify-between gap-4 rounded-xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-700 dark:bg-slate-900">
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <span className={cn('h-2.5 w-2.5 rounded-full', enabled ? 'bg-emerald-500' : 'bg-slate-300 dark:bg-slate-600')} />
          <span className="text-[13.5px] font-medium text-slate-700 dark:text-slate-200">{title}</span>
        </div>
        <p className="mt-1.5 text-[12px] leading-relaxed text-slate-500 dark:text-slate-400">{description}</p>
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={enabled}
        aria-label={title}
        onClick={onToggle}
        className={cn('relative h-6 w-11 shrink-0 rounded-full transition-colors', enabled ? 'bg-indigo-600' : 'bg-slate-300 dark:bg-slate-600')}
      >
        <span className={cn('absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform', enabled && 'translate-x-5')} />
      </button>
    </div>
  );
}

export function ButlerSettingsGroupedDemo() {
  const [openSections, setOpenSections] = useState<Record<SectionId, boolean>>(() => readSectionState(STORAGE_KEY, DEFAULT_OPEN_SECTIONS));
  const [openSkillSections, setOpenSkillSections] = useState<Record<SkillSectionId, boolean>>(() => readSectionState(`${STORAGE_KEY}-skills`, DEFAULT_OPEN_SKILL_SECTIONS));
  const [nickname, setNickname] = useState('boss');
  const [enabledSkills, setEnabledSkills] = useState({ findSkills: true, skillCreator: false, dailyReview: true });
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(openSections));
  }, [openSections]);

  useEffect(() => {
    localStorage.setItem(`${STORAGE_KEY}-skills`, JSON.stringify(openSkillSections));
  }, [openSkillSections]);

  const openCount = useMemo(() => SECTION_IDS.filter(id => openSections[id]).length, [openSections]);
  const allOpen = openCount === SECTION_IDS.length;
  const allClosed = openCount === 0;

  const toggleSection = (id: SectionId) => {
    setOpenSections(prev => ({ ...prev, [id]: !prev[id] }));
  };

  const toggleSkillSection = (id: SkillSectionId) => {
    setOpenSkillSections(prev => ({ ...prev, [id]: !prev[id] }));
  };

  const expandAll = () => {
    setOpenSections(Object.fromEntries(SECTION_IDS.map(id => [id, true])) as Record<SectionId, boolean>);
    setOpenSkillSections(Object.fromEntries(Object.keys(DEFAULT_OPEN_SKILL_SECTIONS).map(id => [id, true])) as Record<SkillSectionId, boolean>);
  };

  const collapseAll = () => {
    setOpenSections(Object.fromEntries(SECTION_IDS.map(id => [id, false])) as Record<SectionId, boolean>);
    setOpenSkillSections(Object.fromEntries(Object.keys(DEFAULT_OPEN_SKILL_SECTIONS).map(id => [id, false])) as Record<SkillSectionId, boolean>);
  };

  const handleSave = () => {
    setSaved(true);
    window.setTimeout(() => setSaved(false), 2200);
  };

  return (
    <div className="h-screen overflow-y-auto bg-[#f7f8fc] text-slate-800 dark:bg-slate-950 dark:text-slate-100">
      <header className="sticky top-0 z-20 border-b border-slate-200/80 bg-white/90 px-6 py-4 backdrop-blur-xl dark:border-slate-800 dark:bg-slate-900/90 sm:px-10">
        <div className="mx-auto flex max-w-4xl items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-indigo-600 text-white shadow-sm">
              <Wand2 size={19} />
            </div>
            <div>
              <p className="text-[11px] font-medium uppercase tracking-[0.16em] text-indigo-600 dark:text-indigo-300">EgoSync Demo</p>
              <h1 className="text-[18px] font-semibold text-slate-800 dark:text-slate-100">管家设置</h1>
            </div>
          </div>
          <span className="hidden rounded-full border border-indigo-100 bg-indigo-50 px-3 py-1.5 text-[12px] font-medium text-indigo-600 sm:inline-flex dark:border-indigo-900/50 dark:bg-indigo-900/30 dark:text-indigo-300">
            分组折叠方案
          </span>
        </div>
      </header>

      <main className="mx-auto max-w-4xl px-5 py-8 sm:px-10 sm:py-10">
        <div className="mb-6 flex flex-col justify-between gap-5 sm:flex-row sm:items-end">
          <div>
            <p className="mb-2 text-[13px] font-medium text-indigo-600 dark:text-indigo-300">更轻松地管理管家</p>
            <h2 className="text-[28px] font-semibold tracking-tight text-slate-900 dark:text-white">把复杂配置收进清晰的分组</h2>
            <p className="mt-2 max-w-2xl text-[13.5px] leading-relaxed text-slate-500 dark:text-slate-400">
              通过一级分组、二级 Skill 分组和配置摘要，快速定位真正需要调整的设置。
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[12px] text-slate-500 shadow-sm dark:border-slate-700 dark:bg-slate-900 dark:text-slate-400">
            <span className="h-2 w-2 rounded-full bg-emerald-500" />
            已展开 {openCount}/{SECTION_IDS.length} 个分组
          </div>
        </div>

        <div className="mb-5 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-slate-200 bg-white px-4 py-3 shadow-sm dark:border-slate-700 dark:bg-slate-900">
          <div className="flex items-center gap-2 text-[12.5px] text-slate-500 dark:text-slate-400">
            <Info size={15} className="text-indigo-500" />
            折叠状态会记住，下次打开时保持当前浏览方式
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={expandAll}
              disabled={allOpen}
              className="rounded-lg border border-slate-200 px-3 py-1.5 text-[12px] font-medium text-slate-600 transition-colors hover:border-indigo-200 hover:bg-indigo-50 hover:text-indigo-600 disabled:cursor-not-allowed disabled:opacity-40 dark:border-slate-700 dark:text-slate-300 dark:hover:border-indigo-700 dark:hover:bg-indigo-900/30 dark:hover:text-indigo-300"
            >
              展开全部
            </button>
            <button
              type="button"
              onClick={collapseAll}
              disabled={allClosed}
              className="rounded-lg border border-slate-200 px-3 py-1.5 text-[12px] font-medium text-slate-600 transition-colors hover:border-indigo-200 hover:bg-indigo-50 hover:text-indigo-600 disabled:cursor-not-allowed disabled:opacity-40 dark:border-slate-700 dark:text-slate-300 dark:hover:border-indigo-700 dark:hover:bg-indigo-900/30 dark:hover:text-indigo-300"
            >
              收起全部
            </button>
          </div>
        </div>

        <div className="space-y-4">
          <SettingsSection
            id="basic"
            title="基础信息"
            description="设置管家对您的称呼和基础个性化信息"
            summary={`称呼：${nickname || '未设置'}`}
            icon={<User size={18} />}
            isOpen={openSections.basic}
            onToggle={() => toggleSection('basic')}
          >
            <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-200" htmlFor="demo-nickname">如何称呼您</label>
            <input
              id="demo-nickname"
              value={nickname}
              onChange={e => setNickname(e.target.value)}
              className="mt-2 w-full rounded-xl border border-slate-200 bg-white px-4 py-2.5 text-[13px] outline-none transition focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100"
            />
            <p className="mt-2 text-[12px] text-slate-400 dark:text-slate-500">管家在对话中称呼您的方式。</p>
          </SettingsSection>

          <SettingsSection
            id="mission"
            title="使命与价值观"
            description="配置管家的长期使命、原则和行为方向"
            summary="已设置 · 自由文本"
            icon={<Target size={18} />}
            isOpen={openSections.mission}
            onToggle={() => toggleSection('mission')}
          >
            <div className="rounded-xl border border-indigo-100 bg-indigo-50/70 px-4 py-4 dark:border-indigo-900/50 dark:bg-indigo-900/20">
              <div className="flex items-start gap-3">
                <CheckCircle2 size={18} className="mt-0.5 shrink-0 text-emerald-600" />
                <div>
                  <p className="text-[13.5px] font-medium text-slate-800 dark:text-slate-100">以家庭为根基，以成长和长期价值为方向</p>
                  <p className="mt-1.5 text-[12.5px] leading-relaxed text-slate-500 dark:text-slate-400">
                    管家会在任务建议、提醒和复盘中参考这份使命宣言。
                  </p>
                </div>
              </div>
            </div>
            <button type="button" onClick={handleSave} className="mt-4 inline-flex items-center gap-1.5 rounded-lg border border-slate-200 px-3.5 py-2 text-[12.5px] font-medium text-slate-600 transition-colors hover:border-indigo-200 hover:bg-indigo-50 hover:text-indigo-600 dark:border-slate-700 dark:text-slate-300 dark:hover:border-indigo-700 dark:hover:bg-indigo-900/30">
              <Target size={14} /> 编辑使命宣言
            </button>
          </SettingsSection>

          <SettingsSection
            id="skills"
            title="能力与 Skill"
            description="管理管家可使用的内置、外部和自定义 Skill"
            summary="已启用 4 项"
            icon={<Sparkles size={18} />}
            isOpen={openSections.skills}
            onToggle={() => toggleSection('skills')}
          >
            <div className="space-y-3">
              <SkillGroup id="builtIn" title="内置 Skill" summary="已启用 2/3 项" isOpen={openSkillSections.builtIn} onToggle={() => toggleSkillSection('builtIn')}>
                <ToggleRow title="find-skills" description="发现并管理可用的 Skill。" enabled={enabledSkills.findSkills} onToggle={() => setEnabledSkills(prev => ({ ...prev, findSkills: !prev.findSkills }))} />
                <ToggleRow title="skill-creator" description="辅助创建和整理自定义 Skill。" enabled={enabledSkills.skillCreator} onToggle={() => setEnabledSkills(prev => ({ ...prev, skillCreator: !prev.skillCreator }))} />
                <ToggleRow title="daily-review" description="支持日常复盘和行动回顾。" enabled={enabledSkills.dailyReview} onToggle={() => setEnabledSkills(prev => ({ ...prev, dailyReview: !prev.dailyReview }))} />
              </SkillGroup>

              <SkillGroup id="opencode" title="opencode 生态 Skill" summary="已发现 6 项 · 2 项已导入" isOpen={openSkillSections.opencode} onToggle={() => toggleSkillSection('opencode')}>
                {['research-assistant', 'weekly-planner'].map(name => (
                  <div key={name} className="flex items-center justify-between gap-3 rounded-lg border border-slate-100 bg-slate-50 px-3 py-2.5 dark:border-slate-700 dark:bg-slate-800">
                    <div className="min-w-0">
                      <p className="text-[13px] font-medium text-slate-700 dark:text-slate-200">{name}</p>
                      <p className="mt-0.5 text-[11.5px] text-slate-400">全局目录 · 已导入</p>
                    </div>
                    <span className="rounded-full bg-emerald-50 px-2 py-0.5 text-[11px] font-medium text-emerald-600 dark:bg-emerald-900/30 dark:text-emerald-300">已启用</span>
                  </div>
                ))}
              </SkillGroup>

              <SkillGroup id="custom" title="自定义 Skill" summary="已导入 2 项" isOpen={openSkillSections.custom} onToggle={() => toggleSkillSection('custom')}>
                <div className="flex items-center justify-between gap-3 rounded-lg border border-slate-100 bg-slate-50 px-3 py-3 dark:border-slate-700 dark:bg-slate-800">
                  <div className="flex min-w-0 items-center gap-2.5">
                    <span className="h-2.5 w-2.5 shrink-0 rounded-full bg-emerald-500" />
                    <div className="min-w-0">
                      <p className="truncate text-[13px] font-medium text-slate-700 dark:text-slate-200">daily-review</p>
                      <p className="mt-0.5 text-[11.5px] text-slate-400">日复盘助手 · 全部角色</p>
                    </div>
                  </div>
                  <button type="button" aria-label="启用 daily-review" role="switch" aria-checked="true" className="relative h-6 w-11 shrink-0 rounded-full bg-indigo-600">
                    <span className="absolute left-0.5 top-0.5 h-5 w-5 translate-x-5 rounded-full bg-white shadow" />
                  </button>
                </div>
                <button type="button" onClick={handleSave} className="inline-flex items-center gap-1.5 text-[12px] font-medium text-indigo-600 hover:text-indigo-700 dark:text-indigo-300">
                  <Plus size={14} /> 添加自定义 Skill
                </button>
              </SkillGroup>
            </div>
          </SettingsSection>

          <SettingsSection
            id="schedule"
            title="自动化与提醒"
            description="设置晨间简报、周复盘和大石头规划提醒"
            summary="3 项已配置"
            icon={<Clock3 size={18} />}
            isOpen={openSections.schedule}
            onToggle={() => toggleSection('schedule')}
          >
            <div className="grid gap-3 sm:grid-cols-3">
              {[
                ['晨间简报', '每天 08:00'],
                ['周复盘', '周日 20:00'],
                ['大石头提醒', '每月 1 日 09:00'],
              ].map(([title, value]) => (
                <div key={title} className="rounded-xl border border-slate-200 bg-slate-50 px-3.5 py-3 dark:border-slate-700 dark:bg-slate-800">
                  <p className="text-[12px] text-slate-500 dark:text-slate-400">{title}</p>
                  <p className="mt-1 text-[13.5px] font-semibold text-slate-700 dark:text-slate-200">{value}</p>
                </div>
              ))}
            </div>
            <button type="button" onClick={handleSave} className="mt-4 inline-flex items-center gap-1.5 rounded-lg border border-slate-200 px-3.5 py-2 text-[12.5px] font-medium text-slate-600 transition-colors hover:border-indigo-200 hover:bg-indigo-50 hover:text-indigo-600 dark:border-slate-700 dark:text-slate-300 dark:hover:border-indigo-700 dark:hover:bg-indigo-900/30">
              <Clock3 size={14} /> 调整提醒时间
            </button>
          </SettingsSection>

          <SettingsSection
            id="archived"
            title="角色管理"
            description="查看并重新启用已经归档的角色"
            summary="3 个已归档"
            icon={<Archive size={18} />}
            isOpen={openSections.archived}
            onToggle={() => toggleSection('archived')}
          >
            <div className="space-y-2">
              {['旅行规划师', '财务顾问', '阅读伙伴'].map(role => (
                <div key={role} className="flex items-center justify-between rounded-xl border border-slate-200 px-4 py-3 dark:border-slate-700">
                  <span className="text-[13px] text-slate-600 dark:text-slate-300">{role}</span>
                  <button type="button" onClick={handleSave} className="text-[12px] font-medium text-indigo-600 hover:text-indigo-700 dark:text-indigo-300">重新启用</button>
                </div>
              ))}
            </div>
          </SettingsSection>
        </div>

        <div className="mt-6 flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-slate-200 bg-white px-5 py-4 shadow-sm dark:border-slate-700 dark:bg-slate-900">
          <div className="flex items-center gap-2 text-[12.5px] text-slate-500 dark:text-slate-400">
            <Save size={15} className="text-indigo-500" />
            这是一个交互 Demo，切换开关和分组即可体验方案
          </div>
          <div className="flex items-center gap-2">
            {saved && <span className="inline-flex items-center gap-1 text-[12px] font-medium text-emerald-600"><Check size={14} /> 操作已完成</span>}
            <button type="button" onClick={handleSave} className="inline-flex items-center gap-1.5 rounded-lg bg-indigo-600 px-4 py-2 text-[12.5px] font-medium text-white shadow-sm transition-colors hover:bg-indigo-700">
              <Save size={14} /> 保存设置
            </button>
          </div>
        </div>
      </main>
    </div>
  );
}

