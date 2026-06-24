export interface Mission {
  id: string;
  content: string | null;
  format: 'free' | 'structured';
  updatedAt: string;
}

export interface InferredValues {
  values: string[];
  summary: string;
  confidence: number;
}

export interface InferenceEligibility {
  eligible: boolean;
  reason: string;
}
