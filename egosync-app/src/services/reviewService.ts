import { invoke } from '@tauri-apps/api/core';
import type { WeeklyReview, BigRockPlanItem, RoleBigRockSuggestions } from '../types/review';
import type { Task } from '../types/task';

export async function getLatestReview(): Promise<WeeklyReview | null> {
  return invoke<WeeklyReview | null>('review_get_latest');
}

export async function getReviewByWeek(weekStart: string): Promise<WeeklyReview | null> {
  return invoke<WeeklyReview | null>('review_get_by_week', { weekStart });
}

export async function generateReviewNow(): Promise<boolean> {
  return invoke<boolean>('review_generate_now');
}

export async function planBigRocks(items: BigRockPlanItem[]): Promise<Task[]> {
  return invoke<Task[]>('review_plan_bigrocks', { items });
}

export async function getBigRockSuggestions(): Promise<RoleBigRockSuggestions[]> {
  return invoke<RoleBigRockSuggestions[]>('review_get_bigrock_suggestions');
}
