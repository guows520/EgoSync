import { useState, type ReactNode } from 'react';
import {
  Check,
  ChevronDown,
  ChevronRight,
  Info,
  Save,
  Search,
  Server,
  Sparkles,
  Upload,
  User,
  Wrench,
  Zap,
} from 'lucide-react';
import { cn } from '../../lib/utils';

type SectionId = 'profile' | 'behavior' | 'builtin-skills' | 'skill-import' | 'mcp' | 'advanced';

type SectionStatus = 'normal' | 'warning' | 'saving';

const SECTION_IDS: SectionId[] = [
  'profile',
  'behavior',
  'builtin-skills',
  'skill-import',
  'mcp',
  'advanced',
];

const INITIAL_EXPANDED: Record<SectionId, boolean> = {
  profile: true,
  behavior: true,
  'builtin-skills': false,
  'skill-import': false,
  mcp: false,
  advanced: false,
};

interface DemoSectionProps {
  id: SectionId;
  title: string;
  description: string;
  summary: string;
  expanded: boolean;
  status?: SectionStatus;
  onToggle: () => void;
  children: ReactNode;
}

function DemoSection({
  id,
  title,
  description,
  summary,
  expanded,
  status = 'normal',
  onToggle,
  children,
}: DemoSectionProps) {
  const contentId = `role-settings-demo-${id}`;

  return (
    <section className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white shadow-sm dark:border-slate-700 dark:bg-slate-800">
      <button
        type="button"
        aria-expanded={expanded}
        aria-controls={contentId}
        onClick={onToggle}
        className="flex w-full items-center gap-3 px-5 py-4 text-left transition-colors hover:bg-slate-50 dark:hover:bg-slate-750"
      >
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-slate-100 text-slate-500 dark:bg-slate-700 dark:text-slate-300">
          {expanded ? <ChevronDown size={17} /> : <ChevronRight size={17} />}
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex flex-wrap items-center gap-2">
            <span className="text-[14px] font-semibold text-slate-800 dark:text-slate-100">{title}</span>
            {status === 'warning' && (
              <span className="rounded-full bg-amber-50 px-2 py-0.5 text-[11px] font-medium text-amber-700 dark:bg-amber-900/30 dark:text-amber-300">
                需要处理
              </span>
            )}
            {status === 'saving' && (
              <span className="rounded-full bg-indigo-50 px-2 py-0.5 text-[11px] font-medium text-indigo-700 dark:bg-indigo-900/30 dark:text-indigo-300">
                保存中...
              </span>
            )}
          </span>
          <span className="mt-1 block text-[12px] leading-relaxed text-slate-500 dark:text-slate-400">{description}</span>
        </span>
        <span className="hidden shrink-0 text-right text-[12px] font-medium text-slate-500 sm:block dark:text-slate-400">{summary}</span>
      </button>
      <div id={contentId} hidden={!expanded} className="border-t border-slate-100 px-5 pb-5 pt-4 dark:border-slate-700">
        {children}
      </div>
    </section>
  );
}

function DemoField({ label, children, hint }: { label: string; children: ReactNode; hint?: string }) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-[13px] font-medium text-slate-600 dark:text-slate-300">{label}</span>
      {children}
      {hint && <span className="mt-1.5 block text-[11.5px] leading-relaxed text-slate-400 dark:text-slate-500">{hint}</span>}
    </label>
  );
}

const inputClassName = 'w-full rounded-lg border border-slate-200 bg-white px-3.5 py-2.5 text-[13px] text-slate-800 outline-none transition focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/15 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100';

export function GroupedSettingsDemo() {
  const [expandedSections, setExpandedSections] = useState<Record<SectionId, boolean>>(INITIAL_EXPANDED);
  const [roleName, setRoleName] = useState('产品经理');
  const [roleGoal, setRoleGoal] = useState('管理产品规划，帮助团队做出更清晰的优先级判断。');
  const [personality, setPersonality] = useState('简洁专业，偏结构化表达，先判断优先级再给建议。');
  const [proactivity, setProactivity] = useState('moderate');
  const [findSkills, setFindSkills] = useState(true);
  const [skillCreator, setSkillCreator] = useState(false);
  const [mcpSearch, setMcpSearch] = useState('');
  const [saved, setSaved] = useState(false);
  const [autoSaved, setAutoSaved] = useState('');

  const allExpanded = SECTION_IDS.every(id => expandedSections[id]);
  const profileDirty = roleName !== '产品经理' || roleGoal !== '管理产品规划，帮助团队做出更清晰的优先级判断。' || personality !== '简洁专业，偏结构化表达，先判断优先级再给建议。';
  const visibleMcpCount = mcpSearch.trim() ? 1 : 2;

  const toggleSection = (id: SectionId) => {
    setExpandedSections(prev => ({ ...prev, [id]: !prev[id] }));
  };

  const setAllSections = (expanded: boolean) => {
    setExpandedSections(Object.fromEntries(SECTION_IDS.map(id => [id, expanded])) as Record<SectionId, boolean>);
  };

  const handleSave = () => {
    setSaved(true);
    window.setTimeout(() => setSaved(false), 1800);
  };

  const handleAutoSave = (message: string) => {
    setAutoSaved(message);
    window.setTimeout(() => setAutoSaved(''), 1800);
  };

  return (
    <div className="h-full overflow-y-auto bg-[#F8F9FA] dark:bg-slate-900">
      <div className="mx-auto min-h-full w-full max-w-[920px] px-5 py-6 sm:px-8 sm:py-8">
        <div className="sticky top-0 z-10 -mx-2 mb-5 rounded-2xl border border-slate-200/80 bg-[#F8F9FA]/90 px-2 py-2 backdrop-blur-md dark:border-slate-700/80 dark:bg-slate-900/90 sm:-mx-3 sm:px-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-[20px] font-semibold tracking-tight text-slate-800 dark:text-slate-100">角色设置</h1>
                <span className="rounded-full bg-indigo-50 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-indigo-600 dark:bg-indigo-900/30 dark:text-indigo-300">Demo</span>
              </div>
              <p className="mt-1 text-[12.5px] text-slate-500 dark:text-slate-400">通过分组管理角色能力，减少长页面滚动和认知负担。</p>
            </div>
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => setAllSections(!allExpanded)}
                className="rounded-lg border border-slate-200 bg-white px-3 py-2 text-[12px] font-medium text-slate-600 transition hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700"
              >
                {allExpanded ? '全部折叠' : '展开全部'}
              </button>
              <button
                type="button"
                onClick={() => setAllSections(false)}
                className="hidden rounded-lg px-3 py-2 text-[12px] font-medium text-slate-500 transition hover:bg-slate-100 sm:block dark:text-slate-400 dark:hover:bg-slate-800"
              >
                折叠全部
              </button>
            </div>
          </div>
        </div>

        <div className="mb-4 flex items-center gap-2 rounded-xl border border-indigo-100 bg-indigo-50/70 px-3.5 py-2.5 text-[12px] leading-relaxed text-indigo-700 dark:border-indigo-800 dark:bg-indigo-900/20 dark:text-indigo-300">
          <Info size={15} className="shrink-0" />
          <span>这是交互 Demo：基础信息使用底部统一保存，主动性、Skill 和 MCP 用即时保存模拟。</span>
        </div>

        <div className="space-y-3">
          <DemoSection
            id="profile"
            title="角色信息"
            description="配置角色名称、图标、目标和表达风格。"
            summary={profileDirty ? '有未保存更改' : '已填写'}
            status={profileDirty ? 'warning' : 'normal'}
            expanded={expandedSections.profile}
            onToggle={() => toggleSection('profile')}
          >
            <div className="grid gap-4 sm:grid-cols-2">
              <DemoField label="名称">
                <input className={inputClassName} value={roleName} onChange={e => setRoleName(e.target.value)} />
              </DemoField>
              <DemoField label="角色类型">
                <div className="flex items-center gap-2 rounded-lg border border-slate-200 bg-slate-50 px-3.5 py-2.5 text-[13px] text-slate-600 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-300">
                  <User size={15} className="text-indigo-500" /> 专业角色
                </div>
              </DemoField>
              <DemoField label="目标" hint="一句话说明这个角色要帮助你达成什么。">
                <textarea className={`${inputClassName} resize-none`} rows={2} value={roleGoal} onChange={e => setRoleGoal(e.target.value)} />
              </DemoField>
              <DemoField label="颜色">
                <div className="flex gap-2 rounded-lg border border-slate-200 bg-white p-2.5 dark:border-slate-700 dark:bg-slate-900">
                  {['#4F46E5', '#0EA5E9', '#10B981', '#F59E0B', '#EF4444'].map(color => (
                    <span key={color} className={cn('h-6 w-6 rounded-full ring-offset-2 ring-offset-white dark:ring-offset-slate-900', color === '#4F46E5' && 'ring-2 ring-indigo-500')} style={{ backgroundColor: color }} />
                  ))}
                </div>
              </DemoField>
            </div>
            <div className="mt-4">
              <DemoField label="角色个性描述" hint="描述语气、判断方式和表达习惯。">
                <textarea className={`${inputClassName} resize-none`} rows={3} value={personality} onChange={e => setPersonality(e.target.value)} />
              </DemoField>
            </div>
          </DemoSection>

          <DemoSection
            id="behavior"
            title="行为与主动性"
            description="控制角色在什么时候主动提醒、建议或介入。"
            summary={proactivity === 'proactive' ? '积极主动' : proactivity === 'passive' ? '被动响应' : '适中'}
            expanded={expandedSections.behavior}
            onToggle={() => toggleSection('behavior')}
          >
            <div className="grid gap-2 sm:grid-cols-3">
              {[
                { id: 'passive', title: '被动响应', desc: '仅在你发起对话时工作' },
                { id: 'moderate', title: '适中', desc: '在关键节点给出适度提醒' },
                { id: 'proactive', title: '积极主动', desc: '主动发现问题并提出建议' },
              ].map(option => (
                <button
                  type="button"
                  key={option.id}
                  onClick={() => { setProactivity(option.id); handleAutoSave('主动性级别已自动保存'); }}
                  className={cn('rounded-xl border p-3 text-left transition', proactivity === option.id ? 'border-indigo-400 bg-indigo-50/70 text-indigo-700 dark:border-indigo-500 dark:bg-indigo-900/20 dark:text-indigo-300' : 'border-slate-200 bg-white text-slate-600 hover:border-indigo-200 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-300')}
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-[13px] font-semibold">{option.title}</span>
                    {proactivity === option.id && <Check size={15} />}
                  </div>
                  <span className="mt-1 block text-[11.5px] leading-relaxed opacity-75">{option.desc}</span>
                </button>
              ))}
            </div>
            {autoSaved && <div className="mt-3 flex items-center gap-1.5 text-[12px] text-emerald-600 dark:text-emerald-400"><Check size={14} /> {autoSaved}</div>}
          </DemoSection>

          <DemoSection
            id="builtin-skills"
            title="内置 Skill"
            description="启用系统提供的基础能力。"
            summary={`已启用 ${(findSkills ? 1 : 0) + (skillCreator ? 1 : 0)} / 2 项`}
            expanded={expandedSections['builtin-skills']}
            onToggle={() => toggleSection('builtin-skills')}
          >
            <div className="space-y-2">
              {[
                { title: 'find-skills', description: '发现并推荐适合当前任务的 Skill。', enabled: findSkills, setEnabled: setFindSkills },
                { title: 'skill-creator', description: '创建或扩展角色需要的新 Skill。', enabled: skillCreator, setEnabled: setSkillCreator },
              ].map(skill => (
                <div key={skill.title} className="flex items-center justify-between gap-4 rounded-xl border border-slate-200 bg-slate-50/70 px-4 py-3 dark:border-slate-700 dark:bg-slate-900/70">
                  <div className="flex min-w-0 items-start gap-3">
                    <Sparkles size={17} className={cn('mt-0.5 shrink-0', skill.enabled ? 'text-indigo-500' : 'text-slate-400')} />
                    <div><div className="text-[13px] font-semibold text-slate-700 dark:text-slate-200">{skill.title}</div><p className="mt-1 text-[11.5px] text-slate-500 dark:text-slate-400">{skill.description}</p></div>
                  </div>
                  <button type="button" role="switch" aria-checked={skill.enabled} onClick={() => { skill.setEnabled(!skill.enabled); handleAutoSave(`${skill.title} 已自动保存`); }} className={cn('relative h-6 w-11 shrink-0 rounded-full transition', skill.enabled ? 'bg-indigo-600' : 'bg-slate-300 dark:bg-slate-600')}>
                    <span className={cn('absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform', skill.enabled ? 'translate-x-5' : 'translate-x-0')} />
                  </button>
                </div>
              ))}
            </div>
          </DemoSection>

          <DemoSection
            id="skill-import"
            title="Skill 来源与导入"
            description="发现外部 Skill，或导入本地自定义 Skill。"
            summary="3 个已启用，2 个自定义"
            expanded={expandedSections['skill-import']}
            onToggle={() => toggleSection('skill-import')}
          >
            <div className="space-y-3">
              <div className="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900">
                <div className="flex items-start justify-between gap-3"><div><div className="flex items-center gap-2 text-[13px] font-semibold text-slate-700 dark:text-slate-200"><Search size={15} className="text-indigo-500" /> opencode Skill</div><p className="mt-1 text-[11.5px] text-slate-500 dark:text-slate-400">扫描项目级与全局 Skill 目录，不下载远程内容。</p></div><button type="button" className="rounded-lg border border-slate-200 px-3 py-1.5 text-[12px] font-medium text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800">发现</button></div>
                <div className="mt-3 flex items-center justify-between rounded-lg bg-slate-50 px-3 py-2 text-[12px] dark:bg-slate-800"><span className="text-slate-600 dark:text-slate-300">发现 2 个候选 Skill</span><button type="button" className="font-medium text-indigo-600 dark:text-indigo-300">展开列表</button></div>
              </div>
              <div className="rounded-xl border border-dashed border-indigo-200 bg-indigo-50/50 p-4 dark:border-indigo-800 dark:bg-indigo-900/20"><div className="flex items-center justify-between gap-3"><div><div className="text-[13px] font-semibold text-slate-700 dark:text-slate-200">自定义 Skill</div><p className="mt-1 text-[11.5px] text-slate-500 dark:text-slate-400">选择包含 SKILL.md 的文件夹后，可按角色启用。</p></div><button type="button" className="inline-flex items-center gap-1.5 rounded-lg bg-indigo-600 px-3 py-2 text-[12px] font-medium text-white hover:bg-indigo-700"><Upload size={14} /> 选择文件夹</button></div></div>
            </div>
          </DemoSection>

          <DemoSection
            id="mcp"
            title="外部工具"
            description="管理当前角色可以使用的 MCP server。"
            summary={`${visibleMcpCount} 个 MCP server`}
            status="normal"
            expanded={expandedSections.mcp}
            onToggle={() => toggleSection('mcp')}
          >
            <div className="rounded-xl border border-slate-200 bg-slate-50/70 p-4 dark:border-slate-700 dark:bg-slate-900/70">
              <div className="flex items-start justify-between gap-3"><div><div className="flex items-center gap-2 text-[13px] font-semibold text-slate-700 dark:text-slate-200"><Server size={15} className="text-indigo-500" /> 当前角色可用 MCP server</div><p className="mt-1 text-[11.5px] text-slate-500 dark:text-slate-400">添加或移除后会立即保存。</p></div><button type="button" className="rounded-lg border border-indigo-200 bg-indigo-50 px-3 py-1.5 text-[12px] font-medium text-indigo-700 dark:border-indigo-800 dark:bg-indigo-900/30 dark:text-indigo-300">添加 MCP</button></div>
              <div className="mt-3 grid gap-2 sm:grid-cols-2"><div className="rounded-lg border border-slate-200 bg-white p-3 dark:border-slate-700 dark:bg-slate-800"><div className="flex items-center gap-2 text-[12.5px] font-medium text-slate-700 dark:text-slate-200"><span className="h-2.5 w-2.5 rounded-full bg-emerald-500" /> 日历 MCP</div><p className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">读取日历安排</p></div><div className="rounded-lg border border-slate-200 bg-white p-3 dark:border-slate-700 dark:bg-slate-800"><div className="flex items-center gap-2 text-[12.5px] font-medium text-slate-700 dark:text-slate-200"><span className="h-2.5 w-2.5 rounded-full bg-emerald-500" /> 邮件 MCP</div><p className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">读取邮件摘要</p></div></div>
              <div className="mt-3"><input className={inputClassName} value={mcpSearch} onChange={e => setMcpSearch(e.target.value)} placeholder="搜索 MCP server（演示摘要变化）" /></div>
            </div>
          </DemoSection>

          <DemoSection
            id="advanced"
            title="高级设置"
            description="低频使用的角色维护操作。"
            summary="归档、删除"
            expanded={expandedSections.advanced}
            onToggle={() => toggleSection('advanced')}
          >
            <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-200 bg-amber-50/70 p-4 dark:border-amber-900 dark:bg-amber-900/20"><div><div className="flex items-center gap-2 text-[13px] font-semibold text-amber-800 dark:text-amber-300"><Zap size={15} /> 危险操作</div><p className="mt-1 text-[11.5px] text-amber-700/80 dark:text-amber-300/70">归档或删除角色前请确认影响范围。</p></div><div className="flex gap-2"><button type="button" className="rounded-lg border border-amber-300 px-3 py-1.5 text-[12px] font-medium text-amber-800 dark:border-amber-700 dark:text-amber-300">归档角色</button><button type="button" className="rounded-lg bg-red-600 px-3 py-1.5 text-[12px] font-medium text-white">删除角色</button></div></div>
          </DemoSection>
        </div>

        <div className="sticky bottom-0 mt-5 flex items-center justify-between gap-3 border-t border-slate-200/80 bg-[#F8F9FA]/95 py-4 backdrop-blur-md dark:border-slate-700/80 dark:bg-slate-900/95">
          <div className="flex items-center gap-2 text-[12px] text-slate-500 dark:text-slate-400"><Wrench size={14} /> {profileDirty ? '基础信息有未保存更改' : '所有基础信息已保存'}</div>
          <button type="button" onClick={handleSave} className={cn('inline-flex items-center gap-2 rounded-xl px-5 py-2.5 text-[13px] font-medium text-white shadow-sm transition', saved ? 'bg-emerald-600' : 'bg-indigo-600 hover:bg-indigo-700')}>
            {saved ? <Check size={16} /> : <Save size={16} />}{saved ? '已保存' : '保存更改'}
          </button>
        </div>
      </div>
    </div>
  );
}

