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

/**
 * 移动视口判定（与 Tailwind max-md:<768px 断点同口径）。
 *
 * 用于「行为随断点分叉」的场景：CSS 类只管显隐，管不了交互语义——
 * 典型场景=tab 全屏化后，记忆来源消息跳转必须先在移动端关掉工作区
 * tab 回到对话（对话区 max-md:hidden 时跳转无可见效果）；桌面双栏
 * 下对话区常驻可见，关 tab 会破坏既有双栏行为，故只在小屏关。
 * 点击时点查（不订阅 change）：一次性导航决策，无需重渲染同步。
 */
export function isMobileViewport(): boolean {
  return typeof window !== 'undefined' && window.matchMedia('(max-width: 767px)').matches;
}
