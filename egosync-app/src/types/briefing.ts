export interface Briefing {
  id: string;
  content: string;
  date: string;
  createdAt: string;
}

export interface BriefingGeneratedPayload {
  briefingId: string;
  date: string;
}
