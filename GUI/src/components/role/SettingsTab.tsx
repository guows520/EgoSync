import { useState } from 'react';
import { Plus, Check } from 'lucide-react';
import { cn } from '../../lib/utils';
import { ProactivityToggle } from './ProactivityToggle';

export function SettingsTab({ role, onUpdateRole }: any) {
  const [roleName, setRoleName] = useState(role?.name || '');
  const [roleGoal, setRoleGoal] = useState('');
  const [roleDesc, setRoleDesc] = useState('');
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    onUpdateRole?.({ id: role.id, name: roleName });
    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  };

  return (
    <div className="space-y-8">
      <div>
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">角色信息</label>
        <div className="space-y-4">
          <div>
            <label className="block text-[13px] font-medium text-slate-600 mb-1.5">名称</label>
            <input type="text" value={roleName} onChange={e => setRoleName(e.target.value)} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
          </div>
          <div>
            <label className="block text-[13px] font-medium text-slate-600 mb-1.5">目标</label>
            <textarea value={roleGoal} onChange={e => setRoleGoal(e.target.value)} placeholder="这个角色要达成的核心目标..." rows={2} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none" />
          </div>
          <div>
            <label className="block text-[13px] font-medium text-slate-600 mb-1.5">职责描述</label>
            <textarea value={roleDesc} onChange={e => setRoleDesc(e.target.value)} placeholder="这个角色负责处理哪些事务..." rows={2} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none" />
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
        <button onClick={handleSave} className={cn("w-full py-3 rounded-xl text-[14px] font-medium transition-all flex items-center justify-center gap-2", saved ? "bg-emerald-600 text-white" : "bg-indigo-600 text-white hover:bg-indigo-700 shadow-sm")}>
          {saved ? <><Check size={16} /> 已保存</> : '保存更改'}
        </button>
      </div>
    </div>
  );
}
