// TODO: 后续 Story 将替换为 Tauri invoke 真实数据

import { Briefcase, Heart, BookOpen } from 'lucide-react';

export const DEFAULT_ROLES = [
  { id: 'pm', name: '产品经理', icon: Briefcase, color: 'bg-indigo-600', text: 'text-indigo-600', tint: 'bg-indigo-50/40', energy: 85, status: 'green' },
  { id: 'family', name: '家庭', icon: Heart, color: 'bg-amber-600', text: 'text-amber-600', tint: 'bg-amber-50/40', energy: 60, status: 'yellow' },
  { id: 'study', name: '学习者', icon: BookOpen, color: 'bg-purple-600', text: 'text-purple-600', tint: 'bg-purple-50/40', energy: 70, status: 'none' }
];

export const ROLE_TASKS: Record<string, {title: string, deadline?: string, isBigRock?: boolean}[]> = {
  pm: [
    { title: '跟进核心OKR指标', deadline: '今日 18:00', isBigRock: true },
    { title: '准备Q3产品路线图评审材料', deadline: '周五' },
  ],
  family: [
    { title: '确认周末科技馆出行时间', deadline: '明天', isBigRock: true },
    { title: '预约下周家长会', deadline: '周三' },
  ],
  study: [
    { title: '完成《系统思考》第7章阅读', deadline: '本周', isBigRock: true },
    { title: '整理学习笔记并归档', deadline: '周日' },
  ],
};

export const MOCK_NOTIFICATIONS = [
  { id: 1, role: '产品经理', color: 'bg-indigo-600', level: 'knock' as const, text: 'Q3 OKR截止日期是明天，还有2项未完成。', time: '10分钟前' },
  { id: 2, role: '家庭', color: 'bg-amber-600', level: 'tap' as const, text: '周末科技馆门票已预约成功，建议提前确认出行时间。', time: '1小时前' },
  { id: 3, role: '学习者', color: 'bg-purple-600', level: 'whisper' as const, text: '《系统思考》阅读进度已达60%，本周目标可达成。', time: '3小时前' },
  { id: 4, role: '管家', color: 'bg-slate-700', level: 'tap' as const, text: '检测到明天日程较紧，建议提前准备。', time: '5小时前' },
];
