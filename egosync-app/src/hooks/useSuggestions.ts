import { useCallback, useEffect, useState } from 'react';
import { suggestionService } from '../services/suggestionService';
import type { SuggestionWithRole } from '../types/suggestion';

const SUGGESTION_LOAD_ERROR = '建议暂时加载失败，请稍后再试';
const SUGGESTION_CONFIRM_ERROR = '确认建议失败，请稍后再试';
const SUGGESTION_REJECT_ERROR = '拒绝建议失败，请稍后再试';

export type RejectReason = 'irrelevant' | 'bad_timing' | 'already_done' | 'other';

export const REJECT_REASONS: { value: RejectReason; label: string }[] = [
  { value: 'irrelevant', label: '不相关' },
  { value: 'bad_timing', label: '时机不对' },
  { value: 'already_done', label: '已完成' },
  { value: 'other', label: '其他' },
];

export function useSuggestions() {
  const [suggestions, setSuggestions] = useState<SuggestionWithRole[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [actionInFlight, setActionInFlight] = useState<string | null>(null);

  const refetch = useCallback(() => {
    setReloadKey(key => key + 1);
  }, []);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);

    void (async () => {
      try {
        const items = await suggestionService.listPending();
        if (!cancelled) setSuggestions(items);
      } catch (e) {
        console.error('加载待处理建议失败:', e);
        if (!cancelled) {
          setSuggestions([]);
          setError(SUGGESTION_LOAD_ERROR);
        }
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [reloadKey]);

  const removeSuggestion = useCallback((id: string) => {
    setSuggestions(prev => prev.filter(s => s.id !== id));
  }, []);

  const confirmSuggestion = useCallback(async (id: string) => {
    setActionInFlight(id);
    try {
      await suggestionService.confirm(id);
    } catch (e) {
      console.error(SUGGESTION_CONFIRM_ERROR, e);
      setError(SUGGESTION_CONFIRM_ERROR);
      throw e;
    } finally {
      setActionInFlight(null);
    }
  }, []);

  const rejectSuggestion = useCallback(async (id: string, reason: string) => {
    setActionInFlight(id);
    try {
      await suggestionService.reject(id, reason);
    } catch (e) {
      console.error(SUGGESTION_REJECT_ERROR, e);
      setError(SUGGESTION_REJECT_ERROR);
      throw e;
    } finally {
      setActionInFlight(null);
    }
  }, []);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  return {
    suggestions,
    isLoading,
    error,
    actionInFlight,
    refetch,
    confirmSuggestion,
    rejectSuggestion,
    removeSuggestion,
    clearError,
  };
}
