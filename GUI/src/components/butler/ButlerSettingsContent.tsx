import { useEffect, useState } from 'react';
import { AlertCircle, Check } from 'lucide-react';
import { getRoleIconComponent, normalizeColorHex } from '../../lib/roleIcons';
import { cn } from '../../lib/utils';
import { appService } from '../../services/appService';
import type { RoleSkillsConfig } from '../../types/role';

const DEFAULT_SKILLS: RoleSkillsConfig = { findSkills: true, skillCreator: false };
const SKILL_OPTIONS: Array<{ key: keyof RoleSkillsConfig; title: string; source: string; description: string }> = [
  {
    key: 'findSkills',
    title: 'find-skills',
    source: 'Vercel 官方',
    description: '发现并推荐适合当前任务的 Skill。',
  },
  {
    key: 'skillCreator',
    title: 'skill-creator',
    source: 'Anthropic 官方',
    description: '创建或扩展角色需要的新 Skill。',
  },
];

export function ButlerSettingsContent({ archivedRoles = [], onRestoreRole }: any) {
  const [mission, setMission] = useState('');
  const [skills, setSkills] = useState<RoleSkillsConfig>(DEFAULT_SKILLS);
  const [pendingSkill, setPendingSkill] = useState<keyof RoleSkillsConfig | null>(null);
  const [settingsSavedMessage, setSettingsSavedMessage] = useState('');
  const [error, setError] = useState('');
  const templates = [
    '家庭优先：家人的健康与陪伴是一切决策的第一优先级。',
    '事业与家庭平衡：在事业高速成长的同时，确保每周至少两个晚上属于家人。',
    '终身学习：持续投入时间学习新知识，每月至少完成一本书或一门课程。'
  ];

  useEffect(() => {
    let cancelled = false;
    appService.getButlerSkills()
      .then(nextSkills => {
        if (!cancelled) setSkills(nextSkills);
      })
      .catch(() => {
        if (!cancelled) setError('Skill 配置加载失败，请稍后重试');
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleSkillToggle = async (key: keyof RoleSkillsConfig) => {
    const nextSkills = { ...skills, [key]: !skills[key] };
    setSkills(nextSkills);
    setPendingSkill(key);
    setSettingsSavedMessage('');
    setError('');
    try {
      const savedSkills = await appService.updateButlerSkills(nextSkills);
      setSkills(savedSkills);
      setSettingsSavedMessage('Skill 配置已保存');
      setTimeout(() => setSettingsSavedMessage(''), 1500);
    } catch {
      setSkills(skills);
      setError('Skill 配置保存失败，请稍后重试');
    } finally {
      setPendingSkill(null);
    }
  };

  return (
    <div className="space-y-8">
      <div>
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">如何称呼您</label>
        <input type="text" defaultValue="boss" className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
        <p className="text-[12px] text-slate-400 mt-2 leading-relaxed">管家在对话中称呼您的方式。</p>
      </div>
      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">您的个人使命宣言</label>
        <textarea
          value={mission}
          onChange={e => setMission(e.target.value)}
          placeholder="个人使命宣言是你人生各角色间的优先级指南。当角色之间发生时间或资源冲突时，管家会依据它来做仲裁建议。"
          rows={4}
          className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none placeholder:text-slate-300"
        />
        <div className="mt-3 space-y-2">
          <p className="text-[12px] text-slate-400 mb-1.5">参考模板（点击填入）：</p>
          {templates.map((t, i) => (
            <button key={i} onClick={() => setMission(t)} className="block w-full text-left px-3 py-2 rounded-lg border border-slate-200 text-[13px] text-slate-600 hover:bg-indigo-50 hover:border-indigo-200 hover:text-indigo-700 transition-colors">
              {t}
            </button>
          ))}
        </div>
      </div>
      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">Skill 配置</label>
        <div className="space-y-3">
          {SKILL_OPTIONS.map(option => {
            const enabled = skills[option.key];
            return (
              <div key={option.key} className="bg-white border border-slate-200 rounded-xl p-4 shadow-sm">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <div className="flex items-center gap-2.5">
                      <span className={cn('w-2.5 h-2.5 rounded-full', enabled ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]' : 'bg-slate-300')} />
                      <span className="text-[14.5px] font-medium text-slate-800">{option.title}</span>
                      <span className="text-[11px] font-medium text-slate-500 bg-slate-100 px-2 py-0.5 rounded-full border border-slate-200">{option.source}</span>
                    </div>
                    <p className="mt-2 text-[12.5px] leading-relaxed text-slate-500">{option.description}</p>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={enabled}
                    disabled={pendingSkill !== null}
                    onClick={() => handleSkillToggle(option.key)}
                    className={cn(
                      'relative h-6 w-11 shrink-0 rounded-full transition-colors disabled:cursor-not-allowed disabled:opacity-60',
                      enabled ? 'bg-indigo-600' : 'bg-slate-300',
                    )}
                  >
                    <span className={cn('absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform', enabled ? 'translate-x-5' : 'translate-x-0')} />
                    <span className="sr-only">{option.title}</span>
                  </button>
                </div>
              </div>
            );
          })}
        </div>
        {settingsSavedMessage && (
          <div className="mt-3 rounded-lg border border-emerald-100 bg-emerald-50 px-3 py-2 text-[13px] text-emerald-700 flex items-center gap-2">
            <Check size={14} /> {settingsSavedMessage}
          </div>
        )}
        {error && (
          <div className="mt-3 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
            <AlertCircle size={14} /> {error}
          </div>
        )}
      </div>
      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">晨间简报时间</label>
        <input type="time" defaultValue="08:00" className="bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
        <p className="text-[12px] text-slate-400 mt-2 leading-relaxed">每天推送晨间简报的时间。</p>
      </div>
      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">周复盘时间</label>
        <div className="flex gap-3">
          <select defaultValue="7" className="bg-white border border-slate-200 rounded-lg px-3 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 outline-none">
            <option value="1">周一</option><option value="2">周二</option><option value="3">周三</option>
            <option value="4">周四</option><option value="5">周五</option><option value="6">周六</option>
            <option value="7">周日</option>
          </select>
          <input type="time" defaultValue="20:00" className="bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
        </div>
        <p className="text-[12px] text-slate-400 mt-2 leading-relaxed">每周触发周复盘的时间。</p>
      </div>

      {archivedRoles.length > 0 && (
        <div className="pt-6 border-t border-slate-200/80">
          <label className="text-[14px] font-semibold text-slate-800 block mb-3">已归档角色</label>
          <div className="space-y-2.5">
            {archivedRoles.map((role: any) => {
              const Icon = getRoleIconComponent(role.icon);
              const roleColor = normalizeColorHex(role.color);
              return (
              <div key={role.id} className="bg-white border border-slate-200 rounded-xl p-4 flex items-center justify-between shadow-sm">
                <div className="flex items-center gap-3">
                  <div className="w-9 h-9 rounded-lg flex items-center justify-center text-white opacity-60" style={{ backgroundColor: roleColor }}>
                    <Icon size={18} />
                  </div>
                  <div>
                    <p className="text-[14px] font-medium text-slate-700">{role.name}</p>
                    <p className="text-[12px] text-slate-400">已归档</p>
                  </div>
                </div>
                <button onClick={() => onRestoreRole(role.id)} className="px-4 py-2 rounded-lg text-[13px] font-medium text-indigo-600 hover:bg-indigo-50 border border-indigo-200 transition-colors">
                  重新启用
                </button>
              </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
