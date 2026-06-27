import { invoke } from '@tauri-apps/api/core';
import type { WeeklyReview } from '../types/review';

export async function getLatestReview(): Promise<WeeklyReview | null> {
  return invoke<WeeklyReview | null>('review_get_latest');
}

export async function getReviewByWeek(weekStart: string): Promise<WeeklyReview | null> {
  return invoke<WeeklyReview | null>('review_get_by_week', { weekStart });
}

export async function generateReviewNow(): Promise<boolean> {
  return invoke<boolean>('review_generate_now');
}
