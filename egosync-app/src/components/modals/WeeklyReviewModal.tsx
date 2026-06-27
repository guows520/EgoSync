import { useState, useEffect, useMemo, useCallback } from 'react';
import { CheckCircle2, ArrowRight, Plus, X, Check } from 'lucide-react';
import { cn } from '../../lib/utils';
import { Modal } from '../layout/Modal';
import { useWeeklyReview } from '../../hooks/useWeeklyReview';
import { useBigRockPlanning } from '../../hooks/useBigRockPlanning';
import { normalizeColorHex } from '../../lib/roleIcons';
import type { Role } from '../../types/role';
import type { BigRockPlanItem, RoleBigRockSuggestions } from '../../types/review';

interface WeeklyReviewModalProps {
  roles: Role[];
  onClose: () => void;
  initialPhase?: 'review' | 'plan';
}

function getWeekStart(): string {
  const today = new Date();
  const day = today.getDay();
  const monday = new Date(today.getFullYear(), today.getMonth(), today.getDate() - (day === 0 ? 6 : day - 1));
  const y = monday.getFullYear();
  const m = String(monday.getMonth() + 1).padStart(2, '0');
  const d = String(monday.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

function getWeekEnd(weekStart: string): string {
  const [y, m, d] = weekStart.split('-').map(Number);
  const sunday = new Date(y, m - 1, d + 6);
  return `${sunday.getFullYear()}-${String(sunday.getMonth() + 1).padStart(2, '0')}-${String(sunday.getDate()).padStart(2, '0')}`;
}

function formatDateRange(weekStart: string): string {
  const [y, m, d] = weekStart.split('-').map(Number);
  const weekEnd = getWeekEnd(weekStart);
  const [, endM, endD] = weekEnd.split('-').map(Number);
  // 跨月时结束日补显月份，避免“6月29日 - 5日”的歧义
  return endM === m
    ? `${y}年${m}月${d}日 - ${endD}日`
    : `${y}年${m}月${d}日 - ${endM}月${endD}日`;
}

interface RolePlanState {
  roleId: string;
  items: string[];
}

export function WeeklyReviewModal({ roles, onClose, initialPhase = 'review' }: WeeklyReviewModalProps) {
  const [phase, setPhase] = useState<'review' | 'plan'>(initialPhase);
  const weekStart = useMemo(() => getWeekStart(), []);
  const dateRangeLabel = useMemo(() => formatDateRange(weekStart), [weekStart]);

  const { review, energyTrendsData, bigrockStatusList, isLoading: isLoadingReview } = useWeeklyReview(weekStart);
  const { suggestions, isLoadingSuggestions, isSaving, error: planError, loadSuggestions, savePlan } = useBigRockPlanning(roles);

  const [planStates, setPlanStates] = useState<RolePlanState[]>([]);

  useEffect(() => {
    if (phase === 'plan') {
      loadSuggestions();
    }
  }, [phase, loadSuggestions]);

  const roleIdsKey = useMemo(() => roles.map(r => r.id).join(','), [roles]);
  useEffect(() => {
    // 仅当角色集合变化时重建规划状态，避免 roles 引用刷新清空用户正在输入的内容
    setPlanStates(prev => {
      const prevById = new Map(prev.map(p => [p.roleId, p]));
      return roles.map(r => prevById.get(r.id) ?? { roleId: r.id, items: [''] });
    });
  }, [roleIdsKey]); // eslint-disable-line react-hooks/exhaustive-deps

  const getSuggestionsForRole = useCallback(
    (roleId: string): RoleBigRockSuggestions | undefined => {
      return suggestions?.find(s => s.roleId === roleId);
    },
    [suggestions]
  );

  const updatePlanItems = (roleId: string, items: string[]) => {
    setPlanStates(prev => prev.map(p => (p.roleId === roleId ? { ...p, items } : p)));
  };

  const adoptSuggestion = (roleId: string, suggestion: string) => {
    setPlanStates(prev =>
      prev.map(p => {
        if (p.roleId !== roleId) return p;
        const emptyIdx = p.items.findIndex(it => it.trim() === '');
        if (emptyIdx >= 0) {
          const newItems = [...p.items];
          newItems[emptyIdx] = suggestion;
          return { ...p, items: newItems };
        }
        return { ...p, items: [...p.items, suggestion] };
      })
    );
  };

  const handleConfirmPlan = async () => {
    const items: BigRockPlanItem[] = [];
    for (const ps of planStates) {
      for (const title of ps.items) {
        if (title.trim()) {
          items.push({ roleId: ps.roleId, title: title.trim() });
        }
      }
    }
    if (items.length === 0) return;
    try {
      await savePlan(items);
      onClose();
    } catch {
      // error state is managed by hook
    }
  };

  const hasReviewData = review !== null;

  return (
    <Modal onClose={onClose} width="w-[900px]">
      <div className="p-8 max-h-[80vh] overflow-y-auto">
        <div className="flex justify-between items-start mb-6">
          <div>
            <h2 className="text-[24px] font-light text-slate-800">{phase === 'review' ? '周复盘' : '规划下周大石头'}</h2>
            <p className="text-[13px] text-slate-500 mt-1">{dateRangeLabel}</p>
          </div>
          <button onClick={onClose} className="p-2 text-slate-400 hover:bg-slate-100 rounded-full transition-colors"><X size={20}/></button>
        </div>

        {phase === 'review' ? (
          <>
            <div className="bg-slate-50 rounded-xl p-5 mb-8">
              {isLoadingReview ? (
                <p className="text-[14px] text-slate-400 leading-[1.8]">正在加载本周复盘...</p>
              ) : hasReviewData ? (
                <p className="text-[14px] text-slate-700 leading-[1.8] whitespace-pre-line">{review.summary}</p>
              ) : (
                <p className="text-[14px] text-slate-400 leading-[1.8]">本周复盘尚未生成，可在设置中手动触发或等待自动生成</p>
              )}
            </div>

            <div className="grid grid-cols-2 gap-8">
              <div>
                <h3 className="text-[13px] font-semibold text-slate-800 mb-4">角色能量趋势</h3>
                {hasReviewData && energyTrendsData && Object.keys(energyTrendsData).length > 0 ? (
                  <EnergyTrendChart energyTrendsData={energyTrendsData} roles={roles} />
                ) : (
                  <div className="h-32 flex items-center justify-center text-[12px] text-slate-400">
                    {hasReviewData ? '暂无能量数据' : '本周复盘尚未生成'}
                  </div>
                )}
              </div>

              <div>
                <h3 className="text-[13px] font-semibold text-slate-800 mb-4">本周大石头</h3>
                <div className="space-y-3 max-h-[200px] overflow-y-auto pr-2">
                  {hasReviewData && bigrockStatusList && bigrockStatusList.length > 0 ? (
                    bigrockStatusList.map(item => (
                      <div key={item.id} className={cn("flex items-center gap-3 text-[13px] text-slate-700", !item.isCompleted && "opacity-60")}>
                        {item.isCompleted ? (
                          <CheckCircle2 size={16} className="text-emerald-500 shrink-0" />
                        ) : (
                          <ArrowRight size={16} className="text-amber-500 shrink-0" />
                        )}
                        <span>{item.title}</span>
                      </div>
                    ))
                  ) : (
                    <div className="text-[12px] text-slate-400 py-8 text-center">
                      {hasReviewData ? '暂无大石头数据' : '本周复盘尚未生成'}
                    </div>
                  )}
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
              {roles.map(role => {
                const ps = planStates.find(p => p.roleId === role.id) || { roleId: role.id, items: [''] };
                const roleSuggestions = getSuggestionsForRole(role.id);
                const colorHex = normalizeColorHex(role.color);
                return (
                  <div key={role.id} className="bg-white border border-slate-200 rounded-xl p-5 shadow-sm">
                    <h4 className="text-[14px] font-semibold mb-3" style={{ color: colorHex }}>{role.name}</h4>
                    {isLoadingSuggestions ? (
                      <div className="mb-3 bg-indigo-50/60 border border-indigo-100 rounded-lg px-3 py-2">
                        <span className="text-[12px] text-slate-400">正在思考建议...</span>
                      </div>
                    ) : roleSuggestions && roleSuggestions.suggestions.length > 0 ? (
                      roleSuggestions.suggestions.map((suggestion, sIdx) => (
                        <div key={sIdx} className="flex items-center gap-2 mb-3 bg-indigo-50/60 border border-indigo-100 rounded-lg px-3 py-2">
                          <span className="text-[12px] text-indigo-700 flex-1">💡 {suggestion}</span>
                          <button
                            onClick={() => adoptSuggestion(role.id, suggestion)}
                            className="text-[12px] font-medium text-indigo-600 hover:text-indigo-800 whitespace-nowrap px-2 py-1 rounded hover:bg-indigo-100 transition-colors"
                          >采纳</button>
                        </div>
                      ))
                    ) : (
                      <div className="mb-3 bg-slate-50 border border-slate-200 rounded-lg px-3 py-2">
                        <span className="text-[12px] text-slate-400">手动填写本周大石头</span>
                      </div>
                    )}
                    <div className="space-y-2.5">
                      {ps.items.map((txt, j) => (
                        <div key={j} className="flex gap-2">
                          <input
                            type="text"
                            value={txt}
                            onChange={e => {
                              const newItems = [...ps.items];
                              newItems[j] = e.target.value;
                              updatePlanItems(role.id, newItems);
                            }}
                            placeholder={`${role.name} 下周重要的事...`}
                            className="flex-1 bg-slate-50 border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none"
                          />
                          {ps.items.length > 1 && (
                            <button
                              onClick={() => updatePlanItems(role.id, ps.items.filter((_, k) => k !== j))}
                              className="text-slate-300 hover:text-red-500 transition-colors shrink-0 self-center"
                            >
                              <X size={16} />
                            </button>
                          )}
                        </div>
                      ))}
                      <button
                        onClick={() => updatePlanItems(role.id, [...ps.items, ''])}
                        className="flex items-center gap-1.5 text-[13px] text-indigo-600 hover:text-indigo-700 font-medium mt-1"
                      >
                        <Plus size={14} /> 添加
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
            {planError && (
              <div className="mt-6 bg-red-50 border border-red-200 rounded-lg px-4 py-2.5 text-[13px] text-red-600">{planError}</div>
            )}
            <div className="mt-8 flex justify-between">
              <button onClick={() => setPhase('review')} className="px-5 py-2.5 border border-slate-300 text-slate-600 rounded-lg text-[13px] font-medium hover:bg-slate-100">返回复盘</button>
              <button
                onClick={handleConfirmPlan}
                disabled={isSaving}
                className="px-6 py-2.5 bg-indigo-600 text-white rounded-xl text-[14px] font-medium hover:bg-indigo-700 transition-colors flex items-center gap-2 disabled:opacity-50"
              >
                <Check size={16} /> {isSaving ? '保存中...' : '确认规划'}
              </button>
            </div>
          </>
        )}
      </div>
    </Modal>
  );
}

function EnergyTrendChart({
  energyTrendsData,
  roles,
}: {
  energyTrendsData: Record<string, { energy: number; energyUpdatedAt?: string }>;
  roles: Role[];
}) {
  const entries = roles
    .filter(r => energyTrendsData[r.id])
    .map(r => ({
      name: r.name,
      energy: energyTrendsData[r.id].energy,
      color: normalizeColorHex(r.color),
    }));

  if (entries.length === 0) {
    return (
      <div className="h-32 flex items-center justify-center text-[12px] text-slate-400">暂无能量数据</div>
    );
  }

  const barWidth = 40;
  const gap = 20;
  const chartHeight = 120;
  const labelHeight = 20;
  const svgWidth = entries.length * (barWidth + gap) - gap;
  const svgHeight = chartHeight + labelHeight;

  return (
    <svg width={svgWidth} height={svgHeight} className="overflow-visible">
      {entries.map((entry, i) => {
        const x = i * (barWidth + gap);
        const barHeight = (entry.energy / 100) * chartHeight;
        const y = chartHeight - barHeight;
        return (
          <g key={i}>
            <rect
              x={x}
              y={y}
              width={barWidth}
              height={barHeight}
              rx={4}
              fill={entry.color}
            />
            <text
              x={x + barWidth / 2}
              y={y - 6}
              textAnchor="middle"
              fontSize={11}
              fontWeight="bold"
              fill={entry.color}
            >
              {entry.energy}%
            </text>
            <text
              x={x + barWidth / 2}
              y={chartHeight + labelHeight - 4}
              textAnchor="middle"
              fontSize={11}
              fill="#94a3b8"
            >
              {entry.name.length > 4 ? entry.name.slice(0, 4) + '…' : entry.name}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
