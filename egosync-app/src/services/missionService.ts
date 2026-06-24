import { invoke } from '@tauri-apps/api/core';
import type { InferenceEligibility, InferredValues, Mission } from '../types/mission';

export const missionService = {
  get: () => invoke<Mission | null>('mission_get'),
  update: (content: string | null, format: 'free' | 'structured') =>
    invoke<Mission>('mission_update', { content, format }),
  inferValues: () => invoke<InferredValues | null>('mission_infer'),
  checkInferenceEligibility: () => invoke<InferenceEligibility>('mission_infer_eligibility'),
};
