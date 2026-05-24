/**
 * 角色图标与颜色白名单。
 *
 * 必须与后端 `GUI/src-tauri/src/services/agent_engine.rs` 中的
 * `SUPPORTED_ICONS` / `SUPPORTED_COLORS` 保持完全同步。
 *
 * 设计目标：
 * 1. 全部采用 Lucide React 的【黑白线框】图标（stroke-only），不使用彩色 emoji。
 * 2. 图标按用途分组，覆盖工作/生活/学习/健康/家庭/兴趣几大维度。
 * 3. 颜色提供 8 种品牌色，覆盖常见情绪/属性。
 */
import type { LucideIcon } from 'lucide-react';
import {
  Briefcase,
  Code,
  BarChart,
  Palette,
  PenTool,
  BookOpen,
  GraduationCap,
  Dumbbell,
  HeartPulse,
  Leaf,
  Home,
  Users,
  Baby,
  Gamepad2,
  Music,
  Camera,
  Plane,
  Utensils,
  Coffee,
  Target,
  Sparkles,
  Lightbulb,
  Compass,
  Wallet,
} from 'lucide-react';

export interface RoleIconOption {
  /** 与后端一致的稳定 ID（lower-kebab-case）。 */
  id: string;
  /** Lucide 组件，渲染时统一 strokeWidth=2 黑白线框。 */
  component: LucideIcon;
  /** 中文标签（用于 modal 选择列表）。 */
  label: string;
}

export interface RoleColorOption {
  /** Hex 大写，与后端白名单一致。 */
  hex: string;
  /** 中文标签。 */
  label: string;
}

export const ROLE_ICONS: RoleIconOption[] = [
  { id: 'briefcase', component: Briefcase, label: '工作' },
  { id: 'code', component: Code, label: '编程' },
  { id: 'chart-bar', component: BarChart, label: '数据' },
  { id: 'palette', component: Palette, label: '设计' },
  { id: 'pen-tool', component: PenTool, label: '写作' },
  { id: 'book-open', component: BookOpen, label: '阅读' },
  { id: 'graduation-cap', component: GraduationCap, label: '学习' },
  { id: 'dumbbell', component: Dumbbell, label: '健身' },
  { id: 'heart-pulse', component: HeartPulse, label: '健康' },
  { id: 'leaf', component: Leaf, label: '自然' },
  { id: 'home', component: Home, label: '家庭' },
  { id: 'users', component: Users, label: '朋友' },
  { id: 'baby', component: Baby, label: '育儿' },
  { id: 'gamepad-2', component: Gamepad2, label: '游戏' },
  { id: 'music', component: Music, label: '音乐' },
  { id: 'camera', component: Camera, label: '摄影' },
  { id: 'plane', component: Plane, label: '旅行' },
  { id: 'utensils', component: Utensils, label: '美食' },
  { id: 'coffee', component: Coffee, label: '休闲' },
  { id: 'target', component: Target, label: '目标' },
  { id: 'sparkles', component: Sparkles, label: '灵感' },
  { id: 'lightbulb', component: Lightbulb, label: '想法' },
  { id: 'compass', component: Compass, label: '探索' },
  { id: 'wallet', component: Wallet, label: '财务' },
];

export const ROLE_COLORS: RoleColorOption[] = [
  { hex: '#4F46E5', label: '靛蓝' },
  { hex: '#0EA5E9', label: '天蓝' },
  { hex: '#10B981', label: '翠绿' },
  { hex: '#F59E0B', label: '琥珀' },
  { hex: '#EF4444', label: '玫红' },
  { hex: '#8B5CF6', label: '紫罗兰' },
  { hex: '#EC4899', label: '粉' },
  { hex: '#64748B', label: '石板灰' },
];

/** 默认 icon 与颜色（LLM 没给或给的不在白名单时回退）。 */
export const DEFAULT_ICON_ID = 'target';
export const DEFAULT_COLOR_HEX = '#4F46E5';

/** 通过 id 查找图标组件，找不到时回退到默认 target。 */
export function getRoleIconComponent(iconId: string | null | undefined): LucideIcon {
  if (!iconId) return Target;
  const opt = ROLE_ICONS.find(o => o.id === iconId);
  return opt ? opt.component : Target;
}

/** 通过 id 查找图标 option（含 label）。 */
export function getRoleIconOption(iconId: string | null | undefined): RoleIconOption {
  if (!iconId) return ROLE_ICONS.find(o => o.id === DEFAULT_ICON_ID)!;
  const opt = ROLE_ICONS.find(o => o.id === iconId);
  return opt ?? ROLE_ICONS.find(o => o.id === DEFAULT_ICON_ID)!;
}

/** 校验某个字符串是否合法 icon id；非法时返回默认值。 */
export function normalizeIconId(iconId: string | null | undefined): string {
  if (!iconId) return DEFAULT_ICON_ID;
  return ROLE_ICONS.some(o => o.id === iconId) ? iconId : DEFAULT_ICON_ID;
}

/** 校验某个字符串是否合法 color hex；非法时返回默认值。 */
export function normalizeColorHex(color: string | null | undefined): string {
  if (!color) return DEFAULT_COLOR_HEX;
  const upper = color.trim().toUpperCase();
  return ROLE_COLORS.some(o => o.hex === upper) ? upper : DEFAULT_COLOR_HEX;
}