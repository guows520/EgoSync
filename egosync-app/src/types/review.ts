export interface ReviewGeneratedPayload {
  reviewId: string;
  weekStart: string;
  weekEnd: string;
}

export interface WeeklyReview {
  id: string;
  weekStart: string;
  weekEnd: string;
  summary: string;
  energyTrends: string;
  bigrockStatus: string;
  newMemoriesCount: number;
  createdAt: string;
}
