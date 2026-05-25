import { useEffect, useMemo, useState } from 'react';
import { Plus, Check, Archive, Trash2, AlertCircle } from 'lucide-react';
import { cn } from '../../lib/utils';
import { ROLE_COLORS, ROLE_ICONS, getRoleIconComponent, normalizeColorHex, normalizeIconId } from '../../lib/roleIcons';
import { roleService } from '../../services/roleService';
import type { Role } from '../../types/role';
import { ProactivityToggle } from './ProactivityToggle';

interface SettingsTabProps {
  role: Role;
  activeRoleCount: number;
  onUpdateRole?: (role: Role) => void;
  onArchiveRole?: (id: string) => Promise<void> | void;
  onDeleteRole?: (id: string) => Promise<void> | void;
}

type DangerAction = 'archive' | 'delete' | null;

const MIN_ACTIVE_ROLE_MESSAGE = '至少保留一个角色';

export function SettingsTab({
  role,
  activeRoleCount,
  onUpdateRole,
  onArchiveRole,
  onDeleteRole,
}: SettingsTabProps) {
  const [roleName, setRoleName] = useState(role.name);
  const [roleGoal, setRoleGoal] = useState(role.goal);
  const [rolePersonalityPrompt, setRolePersonalityPrompt] = useState(role.personalityPrompt);
  const [roleIcon, setRoleIcon] = useState(normalizeIconId(role.icon));
  const [roleColor, setRoleColor] = useState(normalizeColorHex(role.color));
  const [saved, setSaved] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState('');
  const [dangerAction, setDangerAction] = useState<DangerAction>(null);
  const [deleteConfirmName, setDeleteConfirmName] = useState('');
  const [isDangerSubmitting, setIsDangerSubmitting] = useState(false);

  const canRemoveRole = activeRoleCount > 1;
  const deleteNameMatches = deleteConfirmName.trim() === role.name;
  const SelectedIcon = useMemo(() => getRoleIconComponent(roleIcon), [roleIcon]);

  useEffect(() => {
    setRoleName(role.name);
    setRoleGoal(role.goal);
    setRolePersonalityPrompt(role.personalityPrompt);
    setRoleIcon(normalizeIconId(role.icon));
    setRoleColor(normalizeColorHex(role.color));
    setSaved(false);
    setError('');
    setDangerAction(null);
    setDeleteConfirmName('');
  }, [role.id, role.name, role.goal, role.personalityPrompt, role.icon, role.color]);

  const handleSave = async () => {
    const name = roleName.trim();
    if (!name) {
      setError('角色名称不能为空');
      return;
    }

    setIsSaving(true);
    setError('');
    try {
      const updated = await roleService.update(role.id, {
        name,
        icon: roleIcon,
        color: roleColor,
        goal: roleGoal.trim(),
        personalityPrompt: rolePersonalityPrompt.trim(),
      });
      onUpdateRole?.(updated);
      setSaved(true);
      setTimeout(() => setSaved(false), 1500);
    } catch (e) {
      setError(toFriendlyError(e, '保存失败，请稍后重试'));
    } finally {
      setIsSaving(false);
    }
  };

  const handleArchive = async () => {
    setIsDangerSubmitting(true);
    setError('');
    try {
      await onArchiveRole?.(role.id);
      setDangerAction(null);
    } catch (e) {
      setError(toFriendlyError(e, '归档失败，请稍后重试'));
    } finally {
      setIsDangerSubmitting(false);
    }
  };

  const handleDelete = async () => {
    if (!deleteNameMatches) return;
    setIsDangerSubmitting(true);
    setError('');
    try {
      await onDeleteRole?.(role.id);
      setDangerAction(null);
    } catch (e) {
      setError(toFriendlyError(e, '删除失败，请稍后重试'));
    } finally {
      setIsDangerSubmitting(false);
    }
  };

  return (
    <div className="space-y-8">
      <div>
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">角色信息</label>
        <div className="space-y-4">
          <div>
            <label htmlFor="role-name" className="block text-[13px] font-medium text-slate-600 mb-1.5">名称</label>
            <input id="role-name" type="text" value={roleName} onChange={e => setRoleName(e.target.value)} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
          </div>

          <div>
            <label className="block text-[13px] font-medium text-slate-600 mb-2">图标</label>
            <div className="flex items-center gap-3 mb-3">
              <div className="w-11 h-11 rounded-xl flex items-center justify-center text-white" style={{ backgroundColor: roleColor }}>
                <SelectedIcon size={22} strokeWidth={2} />
              </div>
              <span className="text-[13px] text-slate-500">选择一个稳定的角色标识</span>
            </div>
            <div className="grid grid-cols-6 gap-2">
              {ROLE_ICONS.map(option => {
                const Icon = option.component;
                return (
                  <button
                    key={option.id}
                    type="button"
                    title={option.label}
                    onClick={() => setRoleIcon(option.id)}
                    className={cn(
                      'w-10 h-10 rounded-lg border flex items-center justify-center transition-all',
                      roleIcon === option.id ? 'border-indigo-500 bg-indigo-50 text-indigo-600' : 'border-slate-200 bg-white text-slate-500 hover:border-indigo-200 hover:text-indigo-500'
                    )}
                  >
                    <Icon size={18} strokeWidth={2} />
                  </button>
                );
              })}
            </div>
          </div>

          <div>
            <label className="block text-[13px] font-medium text-slate-600 mb-2">颜色</label>
            <div className="grid grid-cols-4 gap-2">
              {ROLE_COLORS.map(option => (
                <button
                  key={option.hex}
                  type="button"
                  onClick={() => setRoleColor(option.hex)}
                  className={cn(
                    'flex items-center gap-2 rounded-lg border px-3 py-2 text-[12px] font-medium transition-all',
                    roleColor === option.hex ? 'border-indigo-500 bg-indigo-50 text-indigo-700' : 'border-slate-200 bg-white text-slate-600 hover:border-indigo-200'
                  )}
                >
                  <span className="w-4 h-4 rounded-full" style={{ backgroundColor: option.hex }} />
                  {option.label}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label htmlFor="role-goal" className="block text-[13px] font-medium text-slate-600 mb-1.5">目标</label>
            <textarea id="role-goal" value={roleGoal} onChange={e => setRoleGoal(e.target.value)} placeholder="这个角色要达成的核心目标..." rows={2} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none" />
          </div>

          <div>
            <label htmlFor="role-personality" className="block text-[13px] font-medium text-slate-600 mb-1.5">角色个性描述</label>
            <textarea
              id="role-personality"
              value={rolePersonalityPrompt}
              onChange={e => setRolePersonalityPrompt(e.target.value)}
              placeholder="描述这个角色下次回复时应采用的语气、判断方式和表达习惯..."
              rows={4}
              className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none"
            />
            <div className="mt-2 rounded-lg bg-slate-50 border border-slate-100 px-3 py-2 text-[12px] leading-relaxed text-slate-500">
              <div>模板参考：</div>
              <div>产品经理：简洁专业，偏结构化表达，先判断优先级再给建议。</div>
              <div>家庭：温暖关怀，偏情感支持，先回应感受再给建议。</div>
              <div>学习者：好奇探索，偏启发式提问，鼓励持续尝试。</div>
            </div>
          </div>
        </div>
      </div>

      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">主动性级别</label>
        <ProactivityToggle />
        <p className="text-[12px] text-slate-400 mt-2.5 leading-relaxed">角色会定时审视目标，主动生成待处理建议卡片。</p>
      </div>

      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">Skill 插件配置</label>
        <div className="space-y-3">
          <div className="bg-white border border-slate-200 rounded-xl p-4 shadow-sm">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2.5">
                <span className="w-2.5 h-2.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]"></span>
                <span className="text-[14.5px] font-medium text-slate-800">Web Search API</span>
              </div>
              <span className="text-[12px] font-medium text-emerald-600 bg-emerald-50 px-2 py-0.5 rounded-full border border-emerald-100">已启用</span>
            </div>
            <input type="password" value="sk-xxxx-xxxx-xxxx-xxxx" readOnly className="w-full text-[13px] bg-slate-50 border border-slate-200 rounded-lg px-3 py-2 text-slate-500 focus:outline-none font-mono" />
          </div>
          <button className="w-full py-3 border border-dashed border-slate-300 rounded-xl text-[13px] font-medium text-slate-500 hover:text-indigo-600 hover:border-indigo-300 hover:bg-indigo-50/50 transition-all flex items-center justify-center gap-1.5">
            <Plus size={16}/> 添加新 Skill
          </button>
        </div>
      </div>

      <div className="pt-6 border-t border-slate-200/80">
        {error && (
          <div className="mb-3 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
            <AlertCircle size={14} /> {error}
          </div>
        )}
        <button onClick={handleSave} disabled={isSaving} className={cn('w-full py-3 rounded-xl text-[14px] font-medium transition-all flex items-center justify-center gap-2 disabled:opacity-60', saved ? 'bg-emerald-600 text-white' : 'bg-indigo-600 text-white hover:bg-indigo-700 shadow-sm')}>
          {saved ? <><Check size={16} /> 已保存</> : isSaving ? '保存中...' : '保存更改'}
        </button>
      </div>

      <div className="pt-6 border-t border-red-100">
        <label className="text-[14px] font-semibold text-red-600 block mb-3">危险区域</label>
        {!canRemoveRole && (
          <p className="mb-3 rounded-lg border border-amber-100 bg-amber-50 px-3 py-2 text-[13px] text-amber-700">{MIN_ACTIVE_ROLE_MESSAGE}</p>
        )}
        <div className="space-y-3">
          <button
            type="button"
            disabled={!canRemoveRole}
            onClick={() => setDangerAction('archive')}
            className="w-full flex items-center justify-between rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-[13px] font-medium text-amber-700 hover:bg-amber-100 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <span className="flex items-center gap-2"><Archive size={15} /> 归档角色</span>
            <span>保留历史数据</span>
          </button>
          <button
            type="button"
            disabled={!canRemoveRole}
            onClick={() => setDangerAction('delete')}
            className="w-full flex items-center justify-between rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-[13px] font-medium text-red-600 hover:bg-red-100 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <span className="flex items-center gap-2"><Trash2 size={15} /> 永久删除</span>
            <span>不可撤销</span>
          </button>
        </div>
      </div>

      {dangerAction && (
        <div className="fixed inset-0 z-[220] flex items-center justify-center bg-slate-900/20 backdrop-blur-sm">
          <div className="w-[380px] rounded-2xl bg-white p-6 shadow-2xl">
            <h3 className="mb-3 text-[16px] font-semibold text-slate-800">
              {dangerAction === 'archive' ? '确认归档角色' : '永久删除角色'}
            </h3>
            <p className="mb-5 text-[14px] leading-relaxed text-slate-600">
              {dangerAction === 'archive'
                ? `归档「${role.name}」后，侧边栏将不再显示该角色，但历史数据会保留。`
                : `删除「${role.name}」会永久移除角色以及关联对话。请输入角色名确认。`}
            </p>
            {dangerAction === 'delete' && (
              <input
                value={deleteConfirmName}
                onChange={e => setDeleteConfirmName(e.target.value)}
                placeholder={role.name}
                className="mb-4 w-full rounded-lg border border-slate-200 px-4 py-2.5 text-[14px] outline-none focus:border-red-400 focus:ring-2 focus:ring-red-500/10"
              />
            )}
            <div className="flex justify-end gap-3">
              <button onClick={() => setDangerAction(null)} className="rounded-lg border border-slate-200 px-5 py-2.5 text-[13px] font-medium text-slate-600 hover:bg-slate-50">取消</button>
              <button
                onClick={dangerAction === 'archive' ? handleArchive : handleDelete}
                disabled={isDangerSubmitting || (dangerAction === 'delete' && !deleteNameMatches)}
                className={cn('rounded-lg px-5 py-2.5 text-[13px] font-medium text-white disabled:cursor-not-allowed disabled:opacity-50', dangerAction === 'archive' ? 'bg-amber-600 hover:bg-amber-700' : 'bg-red-600 hover:bg-red-700')}
              >
                {isDangerSubmitting ? '处理中...' : dangerAction === 'archive' ? '确认归档' : '永久删除'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function toFriendlyError(error: unknown, fallback: string) {
  const text = typeof error === 'string' ? error : JSON.stringify(error);
  if (text.includes(MIN_ACTIVE_ROLE_MESSAGE)) return MIN_ACTIVE_ROLE_MESSAGE;
  if (text.includes('角色名称不能为空')) return '角色名称不能为空';
  return fallback;
}