import { useState, useCallback, useEffect, useRef } from 'react';
import { getBigRockSuggestions, planBigRocks } from '../services/reviewService';
import type { RoleBigRockSuggestions, BigRockPlanItem } from '../types/review';
import type { Task } from '../types/task';
import type { Role } from '../types/role';

export interface UseBigRockPlanningResult {
  suggestions: RoleBigRockSuggestions[] | null;
  isLoadingSuggestions: boolean;
  isSaving: boolean;
  error: string | null;
  loadSuggestions: () => void;
  savePlan: (items: BigRockPlanItem[]) => Promise<Task[]>;
}

export function useBigRockPlanning(_roles: Role[]): UseBigRockPlanningResult {
  const [suggestions, setSuggestions] = useState<RoleBigRockSuggestions[] | null>(null);
  const [isLoadingSuggestions, setIsLoadingSuggestions] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 组件卸载后不再 setState，避免 LLM 请求（最长 60s）期间关闭 Modal 触发警告
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const loadSuggestions = useCallback(() => {
    setIsLoadingSuggestions(true);
    setError(null);

    getBigRockSuggestions()
      .then(data => {
        if (!mountedRef.current) return;
        setSuggestions(data);
      })
      .catch(err => {
        if (!mountedRef.current) return;
        setSuggestions([]);
        setError(err instanceof Error ? err.message : '加载建议失败，请手动填写');
      })
      .finally(() => {
        if (!mountedRef.current) return;
        setIsLoadingSuggestions(false);
      });
  }, []);

  const savePlan = useCallback(async (items: BigRockPlanItem[]): Promise<Task[]> => {
    setIsSaving(true);
    setError(null);
    try {
      const tasks = await planBigRocks(items);
      return tasks;
    } catch (err) {
      if (mountedRef.current) setError(err instanceof Error ? err.message : '保存规划失败');
      throw err;
    } finally {
      if (mountedRef.current) setIsSaving(false);
    }
  }, []);

  return { suggestions, isLoadingSuggestions, isSaving, error, loadSuggestions, savePlan };
}
