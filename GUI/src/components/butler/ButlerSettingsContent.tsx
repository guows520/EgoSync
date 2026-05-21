import { useState } from 'react';
import { cn } from '../../lib/utils';

export function ButlerSettingsContent({ archivedRoles = [], onRestoreRole }: any) {
  const [mission, setMission] = useState('');
  const templates = [
    '家庭优先：家人的健康与陪伴是一切决策的第一优先级。',
    '事业与家庭平衡：在事业高速成长的同时，确保每周至少两个晚上属于家人。',
    '终身学习：持续投入时间学习新知识，每月至少完成一本书或一门课程。'
  ];
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
            {archivedRoles.map((role: any) => (
              <div key={role.id} className="bg-white border border-slate-200 rounded-xl p-4 flex items-center justify-between shadow-sm">
                <div className="flex items-center gap-3">
                  <div className={cn("w-9 h-9 rounded-lg flex items-center justify-center text-white opacity-60", role.color)}>
                    <role.icon size={18} />
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
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
