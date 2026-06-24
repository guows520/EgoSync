import { invoke } from '@tauri-apps/api/core';
import type { Mission } from '../types/mission';

export const missionService = {
  get: () => invoke<Mission | null>('mission_get'),
  update: (content: string | null, format: 'free' | 'structured') =>
    invoke<Mission>('mission_update', { content, format }),
};
