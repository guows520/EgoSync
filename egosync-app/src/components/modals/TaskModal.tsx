import { Target, X } from 'lucide-react';
import { Modal } from '../layout/Modal';

export function TaskModal({ onClose }: any) {
  return (
    <Modal onClose={onClose} width="w-[500px]">
      <div className="p-6">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-[18px] font-semibold text-slate-800">新建任务</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600"><X size={20}/></button>
        </div>
        
        <div className="space-y-5">
          <div>
            <label className="block text-[13px] font-medium text-slate-700 mb-1.5">任务内容</label>
            <input type="text" placeholder="例如：准备Q3 OKR规划" className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
          </div>
          
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-[13px] font-medium text-slate-700 mb-1.5">四象限分类</label>
              <select className="w-full bg-white border border-slate-200 rounded-lg px-3 py-2 text-[13px] focus:ring-2 focus:ring-indigo-500/20 outline-none">
                <option>Q1: 重要且紧急</option>
                <option>Q2: 重要不紧急</option>
                <option>Q3: 紧急不重要</option>
                <option>Q4: 不重要不紧急</option>
              </select>
            </div>
            <div>
              <label className="block text-[13px] font-medium text-slate-700 mb-1.5">截止时间</label>
              <input type="date" className="w-full bg-white border border-slate-200 rounded-lg px-3 py-2 text-[13px] focus:ring-2 focus:ring-indigo-500/20 outline-none text-slate-600" />
            </div>
          </div>

          <label className="flex items-center gap-3 p-3 border border-amber-200 bg-amber-50/50 rounded-lg cursor-pointer">
            <input type="checkbox" className="w-4 h-4 text-amber-600 rounded border-amber-300 focus:ring-amber-500" />
            <div className="flex-1">
              <div className="text-[13px] font-semibold text-amber-800">标记为本周大石头</div>
              <div className="text-[11px] text-amber-600/80">在周规划中优先受到系统时间保护</div>
            </div>
            <Target size={20} className="text-amber-500/50" />
          </label>
        </div>

        <div className="mt-8 flex justify-end gap-3">
          <button onClick={onClose} className="px-5 py-2.5 border border-slate-200 text-slate-600 rounded-lg text-[13px] font-medium hover:bg-slate-50">取消</button>
          <button onClick={onClose} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700">保存任务</button>
        </div>
      </div>
    </Modal>
  );
}
