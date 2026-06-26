import { invoke } from '@tauri-apps/api/core';
import type { Briefing } from '../types/briefing';

export async function getLatest(): Promise<Briefing | null> {
  return invoke<Briefing | null>('briefing_get_latest');
}

export async function generateNow(): Promise<boolean> {
  return invoke<boolean>('briefing_generate_now');
}
