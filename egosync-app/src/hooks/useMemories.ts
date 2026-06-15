import { useCallback, useEffect, useState } from 'react';
import { memoryService } from '../services/memoryService';
import type { Memory, MemoryCategory } from '../types/memory';

interface UseMemoriesOptions {
  roleId: string | null;
  includeRoleMemories?: boolean;
  category?: MemoryCategory;
}

export function useMemories({ roleId, includeRoleMemories = false, category }: UseMemoriesOptions) {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [loadedRequestKey, setLoadedRequestKey] = useState<string | null>(null);
  const requestKey = [includeRoleMemories ? 'all' : 'role', roleId ?? '', category ?? '', reloadKey].join('|');

  const refetch = useCallback(() => {
    setReloadKey(key => key + 1);
  }, []);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);

    const options = category ? { category } : undefined;
    const request = includeRoleMemories
      ? memoryService.listAll(options)
      : memoryService.list(roleId, options);

    request
      .then(list => {
        if (!cancelled) {
          setMemories(list);
          setLoadedRequestKey(requestKey);
        }
      })
      .catch(e => {
        console.error('加载记忆失败:', e);
        if (!cancelled) {
          setMemories([]);
          setError('记忆暂时加载失败，请稍后再试');
          setLoadedRequestKey(requestKey);
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [category, includeRoleMemories, requestKey, roleId]);

  const hasCurrentData = loadedRequestKey === requestKey;

  return {
    memories: hasCurrentData ? memories : [],
    isLoading: isLoading || !hasCurrentData,
    error: hasCurrentData ? error : null,
    refetch,
  };
}
