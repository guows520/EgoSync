import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatDeadline(deadline: string): string {
  const match = deadline.match(/^(\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2}))?/);
  if (!match) return deadline;
  const [, , mo, dd, hh, mm] = match;
  if (hh && mm) {
    return `${mo}-${dd} ${hh}:${mm}`;
  }
  return `${mo}-${dd}`;
}
