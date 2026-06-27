import { useState, useEffect } from 'react';
import { getReviewByWeek } from '../services/reviewService';
import type { WeeklyReview } from '../types/review';

export interface EnergyTrendsData {
  energy: number;
  energyUpdatedAt?: string;
}

export interface BigRockStatusItem {
  id: string;
  title: string;
  isCompleted: boolean;
  completedAt: string | null;
  roleName: string | null;
}

export interface UseWeeklyReviewResult {
  review: WeeklyReview | null;
  energyTrendsData: Record<string, EnergyTrendsData> | null;
  bigrockStatusList: BigRockStatusItem[] | null;
  isLoading: boolean;
  error: string | null;
}

export function useWeeklyReview(weekStart: string | null): UseWeeklyReviewResult {
  const [review, setReview] = useState<WeeklyReview | null>(null);
  const [energyTrendsData, setEnergyTrendsData] = useState<Record<string, EnergyTrendsData> | null>(null);
  const [bigrockStatusList, setBigrockStatusList] = useState<BigRockStatusItem[] | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!weekStart) {
      setReview(null);
      setEnergyTrendsData(null);
      setBigrockStatusList(null);
      setIsLoading(false);
      setError(null);
      return;
    }

    let cancelled = false;
    setIsLoading(true);
    setError(null);

    getReviewByWeek(weekStart)
      .then(data => {
        if (cancelled) return;
        setReview(data);
        if (data) {
          try {
            setEnergyTrendsData(JSON.parse(data.energyTrends));
          } catch {
            setEnergyTrendsData(null);
          }
          try {
            setBigrockStatusList(JSON.parse(data.bigrockStatus));
          } catch {
            setBigrockStatusList(null);
          }
        } else {
          setEnergyTrendsData(null);
          setBigrockStatusList(null);
        }
      })
      .catch(err => {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : '加载复盘数据失败');
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [weekStart]);

  return { review, energyTrendsData, bigrockStatusList, isLoading, error };
}
