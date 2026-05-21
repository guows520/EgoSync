import { AlertTriangle, Heart, Briefcase, Check, X } from 'lucide-react';
import { Modal } from '../layout/Modal';

export function ArbitrationModal({ onClose }: any) {
  return (
    <Modal onClose={onClose} width="w-[600px]">
      <div className="flex flex-col">
        <div className="px-6 py-4 border-b border-slate-200 flex items-center justify-between bg-slate-50/50">
          <h2 className="text-[16px] font-semibold flex items-center gap-2 text-slate-800"><AlertTriangle size={18} className="text-amber-500"/> 时间冲突仲裁</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600"><X size={20}/></button>
        </div>

        <div className="p-6">
          <div className="flex gap-4 mb-5">
            <div className="flex-1 bg-indigo-50/50 border border-indigo-100 rounded-xl p-4">
              <h4 className="text-[13px] font-semibold text-indigo-800 mb-2 flex items-center gap-1.5"><Briefcase size={14}/> 产品经理</h4>
              <p className="text-[13px] text-slate-600 leading-relaxed">周五下午 15:00 需要参加产品架构评审会，预计 1.5 小时。</p>
              <span className="inline-block mt-2 px-2 py-0.5 rounded text-[11px] font-semibold bg-blue-50 text-blue-600 border border-blue-100">Q2 重要不紧急</span>
            </div>
            <div className="flex items-center text-slate-300 font-bold italic">VS</div>
            <div className="flex-1 bg-amber-50/50 border border-amber-100 rounded-xl p-4">
              <h4 className="text-[13px] font-semibold text-amber-800 mb-2 flex items-center gap-1.5"><Heart size={14}/> 家庭</h4>
              <p className="text-[13px] text-slate-600 leading-relaxed">周五下午 15:30 答应了去接孩子放学并参加家长会。</p>
              <span className="inline-block mt-2 px-2 py-0.5 rounded text-[11px] font-semibold bg-red-50 text-red-600 border border-red-100">Q1 重要且紧急</span>
            </div>
          </div>

          <div className="bg-slate-50 border border-slate-200 rounded-lg p-3 mb-5 text-[13px] text-slate-600 italic leading-relaxed">
            <span className="text-slate-400 text-[11px] font-bold uppercase tracking-wider block mb-1">使命宣言依据</span>
            「家庭优先：家人的健康与陪伴是一切决策的第一优先级。」
          </div>

          <div className="space-y-3 mb-5">
            <div className="bg-emerald-50 border border-emerald-100 rounded-xl p-4">
              <h4 className="text-[13px] font-semibold text-emerald-800 mb-1.5 flex items-center gap-1.5">💡 方案 A（推荐）</h4>
              <p className="text-[13px] text-emerald-900/80 leading-relaxed">优先保证家长会。将评审会提前至 13:30（会议室空闲），已为你拟好申请邮件。</p>
            </div>
            <div className="bg-blue-50/50 border border-blue-100 rounded-xl p-4">
              <h4 className="text-[13px] font-semibold text-blue-800 mb-1.5 flex items-center gap-1.5">🔄 方案 B</h4>
              <p className="text-[13px] text-blue-900/80 leading-relaxed">评审会保持原时间，但委托同事代为主持前30分钟，你15:30到场做关键决策部分，16:00离开接孩子。</p>
            </div>
          </div>

          <p className="text-[12px] text-slate-400 text-center mb-2">最终决定权在你手中。</p>
        </div>

        <div className="px-6 py-4 border-t border-slate-200 bg-slate-50 flex justify-end gap-3">
          <button onClick={onClose} className="px-5 py-2.5 border border-slate-300 text-slate-600 rounded-lg text-[13px] font-medium hover:bg-slate-100">延后处理</button>
          <button onClick={onClose} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700 flex items-center gap-2"><Check size={16}/> 采纳建议</button>
        </div>
      </div>
    </Modal>
  );
}
