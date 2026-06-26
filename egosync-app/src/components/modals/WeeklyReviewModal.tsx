import { useState } from 'react';
import { CheckCircle2, ArrowRight, Plus, X, Check } from 'lucide-react';
import { cn } from '../../lib/utils';
import { Modal } from '../layout/Modal';

export function WeeklyReviewModal({ roles: _roles, onClose, initialPhase = 'review' }: any) {
  const [phase, setPhase] = useState<'review' | 'plan'>(initialPhase);
  const [planItems, setPlanItems] = useState([
    { role: '产品经理', color: 'indigo', items: [''], suggestion: '上周「竞品分析」已完成，建议本周聚焦Q3路线图定稿' },
    { role: '家庭', color: 'amber', items: [''], suggestion: '本周末孩子有手工比赛，建议预留周六上午陪练' },
    { role: '学习者', color: 'purple', items: [''], suggestion: '《系统思考》延期了，建议本周每天30分钟继续推进' }
  ]);

  return (
    <Modal onClose={onClose} width="w-[900px]">
      <div className="p-8 max-h-[80vh] overflow-y-auto">
        <div className="flex justify-between items-start mb-6">
          <div>
            <h2 className="text-[24px] font-light text-slate-800">{phase === 'review' ? '周复盘' : '规划下周大石头'}</h2>
            <p className="text-[13px] text-slate-500 mt-1">2026年5月12日 - 18日</p>
          </div>
          <button onClick={onClose} className="p-2 text-slate-400 hover:bg-slate-100 rounded-full transition-colors"><X size={20}/></button>
        </div>

        {phase === 'review' ? (
          <>
            <div className="bg-slate-50 rounded-xl p-5 mb-8">
              <p className="text-[14px] text-slate-700 leading-[1.8]">
                这周你在3个维度都有进展。<strong className="text-indigo-700">产品经理</strong>角色完成了竞品分析和两次评审准备，效率很高。<strong className="text-amber-700">家庭</strong>角色帮你安排了周末科技馆活动，孩子们很开心。<strong className="text-purple-700">学习者</strong>角色稳步推进《系统思考》的阅读。
              </p>
            </div>

            <div className="grid grid-cols-2 gap-8">
              <div>
                <h3 className="text-[13px] font-semibold text-slate-800 mb-4">角色能量趋势</h3>
                <div className="flex items-end gap-4 h-32 border-b border-slate-200 pb-2">
                  <div className="flex-1 bg-indigo-500 rounded-t-md relative group" style={{ height: '85%' }}>
                    <span className="absolute -top-6 left-1/2 -translate-x-1/2 text-[11px] font-bold text-indigo-600 opacity-0 group-hover:opacity-100 transition-opacity">85%</span>
                  </div>
                  <div className="flex-1 bg-amber-500 rounded-t-md relative group" style={{ height: '60%' }}>
                    <span className="absolute -top-6 left-1/2 -translate-x-1/2 text-[11px] font-bold text-amber-600 opacity-0 group-hover:opacity-100 transition-opacity">60%</span>
                  </div>
                  <div className="flex-1 bg-purple-500 rounded-t-md relative group" style={{ height: '70%' }}>
                    <span className="absolute -top-6 left-1/2 -translate-x-1/2 text-[11px] font-bold text-purple-600 opacity-0 group-hover:opacity-100 transition-opacity">70%</span>
                  </div>
                </div>
                <div className="flex gap-4 mt-2 text-[11px] text-slate-400 text-center">
                  <div className="flex-1">PM</div>
                  <div className="flex-1">家庭</div>
                  <div className="flex-1">学习</div>
                </div>
              </div>

              <div>
                <h3 className="text-[13px] font-semibold text-slate-800 mb-4">本周大石头</h3>
                <div className="space-y-3 max-h-[200px] overflow-y-auto pr-2">
                  <div className="flex items-center gap-3 text-[13px] text-slate-700">
                    <CheckCircle2 size={16} className="text-emerald-500 shrink-0" />
                    <span>竞品分析报告完成</span>
                  </div>
                  <div className="flex items-center gap-3 text-[13px] text-slate-700">
                    <CheckCircle2 size={16} className="text-emerald-500 shrink-0" />
                    <span>周末家庭活动安排</span>
                  </div>
                  <div className="flex items-center gap-3 text-[13px] text-slate-700 opacity-60">
                    <ArrowRight size={16} className="text-amber-500 shrink-0" />
                    <span>《系统思考》阅读（延至下周）</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="mt-10 flex justify-center">
              <button onClick={() => setPhase('plan')} className="px-6 py-2.5 bg-indigo-600 text-white rounded-xl text-[14px] font-medium hover:bg-indigo-700 transition-colors flex items-center gap-2"><ArrowRight size={16} /> 规划下周大石头</button>
            </div>
          </>
        ) : (
          <>
            <div className="bg-indigo-50 border border-indigo-100 rounded-xl p-4 mb-6 text-[13px] text-indigo-800 leading-relaxed">
              为每个角色设定下周最重要的大石头，系统会在日程中优先为它们保留时间。
            </div>
            <div className="space-y-5">
              {planItems.map((item, i) => (
                <div key={i} className="bg-white border border-slate-200 rounded-xl p-5 shadow-sm">
                  <h4 className={cn("text-[14px] font-semibold mb-3", `text-${item.color}-700`)}>{item.role}</h4>
                  {item.suggestion && (
                    <div className="flex items-center gap-2 mb-3 bg-indigo-50/60 border border-indigo-100 rounded-lg px-3 py-2">
                      <span className="text-[12px] text-indigo-700 flex-1">💡 {item.suggestion}</span>
                      <button onClick={() => { const next = [...planItems]; const items = [...next[i].items]; items[0] = item.suggestion; next[i] = {...item, items, suggestion: ''}; setPlanItems(next); }} className="text-[12px] font-medium text-indigo-600 hover:text-indigo-800 whitespace-nowrap px-2 py-1 rounded hover:bg-indigo-100 transition-colors">采纳</button>
                    </div>
                  )}
                  <div className="space-y-2.5">
                    {item.items.map((txt, j) => (
                      <div key={j} className="flex gap-2">
                        <input
                          type="text"
                          value={txt}
                          onChange={e => { const next = [...planItems]; const items = [...next[i].items]; items[j] = e.target.value; next[i] = {...item, items}; setPlanItems(next); }}
                          placeholder={`${item.role} 下周重要的事...`}
                          className="flex-1 bg-slate-50 border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none"
                        />
                        {item.items.length > 1 && (
                          <button onClick={() => { const next = [...planItems]; next[i] = {...item, items: item.items.filter((_: any, k: number) => k !== j)}; setPlanItems(next); }} className="text-slate-300 hover:text-red-500 transition-colors shrink-0 self-center">
                            <X size={16} />
                          </button>
                        )}
                      </div>
                    ))}
                    <button onClick={() => { const next = [...planItems]; next[i] = {...item, items: [...item.items, '']}; setPlanItems(next); }} className="flex items-center gap-1.5 text-[13px] text-indigo-600 hover:text-indigo-700 font-medium mt-1">
                      <Plus size={14} /> 添加
                    </button>
                  </div>
                </div>
              ))}
            </div>
            <div className="mt-8 flex justify-between">
              <button onClick={() => setPhase('review')} className="px-5 py-2.5 border border-slate-300 text-slate-600 rounded-lg text-[13px] font-medium hover:bg-slate-100">返回复盘</button>
              <button onClick={onClose} className="px-6 py-2.5 bg-indigo-600 text-white rounded-xl text-[14px] font-medium hover:bg-indigo-700 transition-colors flex items-center gap-2"><Check size={16} /> 确认规划</button>
            </div>
          </>
        )}
      </div>
    </Modal>
  );
}
