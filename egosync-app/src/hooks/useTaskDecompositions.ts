import { useCallback, useEffect, useState } from 'react';
import { taskDecompositionService } from '../services/taskDecompositionService';
import type { TaskDecompositionProposal } from '../types/taskDecomposition';

const LOAD_ERROR = '任务拆分提案暂时加载失败，请稍后再试';
const ACTION_ERROR = '处理任务拆分提案失败，请稍后再试';

export function useTaskDecompositions(conversationId: string | null) {
  const [proposals, setProposals] = useState<TaskDecompositionProposal[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    if (!conversationId) {
      setProposals([]);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);
    void taskDecompositionService.listPending(conversationId)
      .then(items => {
        if (!cancelled) setProposals(items);
      })
      .catch((e: unknown) => {
        console.error('加载任务拆分提案失败:', e);
        if (!cancelled) {
          setProposals([]);
          setError(LOAD_ERROR);
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [conversationId, reloadKey]);

  const removeProposal = useCallback((id: string) => {
    setProposals(current => current.filter(proposal => proposal.id !== id));
  }, []);

  const acceptProposal = useCallback(async (id: string) => {
    setError(null);
    try {
      await taskDecompositionService.accept(id);
      removeProposal(id);
    } catch (e) {
      console.error(ACTION_ERROR, e);
      setError(ACTION_ERROR);
      throw e;
    }
  }, [removeProposal]);

  const keepSingleProposal = useCallback(async (id: string) => {
    setError(null);
    try {
      await taskDecompositionService.keepSingle(id);
      removeProposal(id);
    } catch (e) {
      console.error(ACTION_ERROR, e);
      setError(ACTION_ERROR);
      throw e;
    }
  }, [removeProposal]);

  const refetch = useCallback(() => {
    setReloadKey(key => key + 1);
  }, []);

  return {
    proposals,
    isLoading,
    error,
    refetch,
    acceptProposal,
    keepSingleProposal,
  };
}
