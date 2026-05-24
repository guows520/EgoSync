# Story 1.9: 用户在管家视角看到呼吸动画的角色侧边栏图标

Status: done

## Story

As a 用户,
I want 侧边栏的角色图标有呼吸动画并显示真实状态,
So that 能感受到角色是"活的"实体。

## Acceptance Criteria

1. **AC-1 真实角色数据**：用户已创建 1+ 角色，打开应用 → 侧边栏显示真实角色图标（来自 `role::list` command，不是 `DEFAULT_ROLES` 硬编码）。
2. **AC-2 呼吸动画**：任意角色图标 → CSS keyframe 呼吸动画（opacity 0.65→1, 3s ease-in-out 循环），60fps 无掉帧（Chrome DevTools Performance 验证），动画使用 GPU 加速（`will-change: opacity`）。
3. **AC-3 减弱动效**：`prefers-reduced-motion: reduce` → 呼吸动画停止，图标保持静态 opacity 1。
4. **AC-4 屏幕阅读器**：焦点到角色图标 → 朗读 `aria-label`（如"产品经理 - 能量值 85%"）。
5. **AC-5 键盘导航**：焦点在侧边栏 → 上下键切换角色图标，Enter 进入角色视图。
6. **AC-6 组件提取**：`components/layout/RoleSidebarIcon.tsx` 从 Sidebar 抽取为独立组件。
7. **AC-7 Emoji 图标渲染**：后端角色 `icon` 字段为 emoji 字符串（如 "📋"），前端正确渲染为 emoji 文字（而非 Lucide React 组件）。
8. **AC-8 现有测试通过**：`cargo test` + `npm run test:frontend` 全部通过。

## Tasks / Subtasks

### Phase 1: 提取 RoleSidebarIcon 组件 (AC: #2, #3, #4, #5, #6, #7)

- [x] **T1.1** 创建 `GUI/src/components/layout/RoleSidebarIcon.tsx`
- [x] **T1.2** 在 `index.css` 的 `.breathe` keyframe 规则中添加 `will-change: opacity;` 启用 GPU 加速
- [x] **T1.3** 验证 `prefers-reduced-motion` 已覆盖 `.breathe` 动画

### Phase 2: Sidebar 重构 — 使用 RoleSidebarIcon + 真实数据适配 (AC: #1, #5, #6, #7)

- [x] **T2.1** 修改 `GUI/src/components/layout/Sidebar.tsx` — 替换为 RoleSidebarIcon 组件
- [x] **T2.2** 添加侧边栏 `role="navigation"` + `aria-label="角色导航"` 语义化属性
- [x] **T2.3** 实现键盘导航：ArrowUp/ArrowDown 切换焦点

### Phase 3: 角色数据桥接 — mock 与真实数据兼容 (AC: #1, #7)

- [x] **T3.1** RoleSidebarIcon 兼容 emoji 字符串和 Lucide 组件两种 icon 格式
- [x] **T3.2** RoleSidebarIcon 兼容 hex 色值和 Tailwind class 两种 color 格式
- [x] **T3.3** 能量状态小点基于 energy 值动态计算（不依赖 mock status 字段）

### Phase 4: 测试 (AC: #8)

- [x] **T4.1** 创建 `GUI/src/components/layout/RoleSidebarIcon.test.tsx` — 12 个测试用例
- [x] **T4.2** 运行 `npm run test:frontend` (13 passed) + `cargo test` (39 passed)：全部通过

## Dev Notes

### ⚠️ 致命陷阱清单

#### 陷阱 1：角色 icon 类型不匹配 — 这是本 Story 最关键的问题

当前 `DEFAULT_ROLES` mock 数据使用 Lucide React 组件作为 icon：
```typescript
// constants/mockData.ts 当前格式
{ id: 'pm', name: '产品经理', icon: Briefcase, color: 'bg-indigo-600', ... }
```
Sidebar.tsx 渲染为：`<role.icon size={22} />` — 这要求 icon 是 React 组件。

后端 `Role` 模型的 icon 是 emoji 字符串：
```typescript
// types/role.ts
{ icon: string; } // 如 "📋"
```
Rust 端 `roles` 表 `icon TEXT NOT NULL DEFAULT '🎯'`。

**解决方案：** 将 `DEFAULT_ROLES` mock 数据也改为 emoji 字符串 + hex 颜色值，与后端格式统一。Sidebar/RoleSidebarIcon 渲染 emoji 文字而非 Lucide 组件。`ICON_OPTIONS`（用于 AddRoleModal 选择图标）保留 Lucide 组件用于选择器 UI，但选中后存储为 emoji。

#### 陷阱 2：角色 color 格式不匹配

`DEFAULT_ROLES` 的 color 是 Tailwind class：`bg-indigo-600`
后端 `Role` 的 color 是 hex 值：`#4F46E5`

Sidebar 选中态当前用 `className={role.color}` 设背景色 — 这对 Tailwind class 有效，但对 hex 无效。

**解决方案：** 选中态改用 `style={{ backgroundColor: role.color }}` inline style。同时 `DEFAULT_ROLES` 的 color 也改为 hex 值。

#### 陷阱 3：角色 status 字段语义不同

`DEFAULT_ROLES` 的 status 是 UI 状态：`'green' | 'yellow' | 'none'`（控制状态小点颜色）。
后端 `Role` 的 status 是生命周期状态：`'active' | 'archived'`。

侧边栏状态小点应基于 `energy` 值动态计算，而非依赖 mock 的 `status` 字段。

**解决方案：** RoleSidebarIcon 根据 `energy` 值动态决定小点：
- `energy >= 80` → 绿色（翠绿 `#10B981`）
- `energy >= 40` → 琥珀色（`#F59E0B`）
- `energy < 40` → 暗淡灰（`#9CA3AF`，不用红色）

#### 陷阱 4：呼吸动画已存在但缺少 GPU 加速

`index.css` 已定义 `.breathe` 动画和 `prefers-reduced-motion` 支持。但缺少 `will-change: opacity` 声明。需要添加到 `.breathe` class 中：

```css
.breathe {
  animation: breathe var(--duration-breath) ease-in-out infinite;
  will-change: opacity;
}
```

#### 陷阱 5：App.tsx roles state 的类型断言

当前 `App.tsx` 在加载真实角色时使用 `as any` 类型断言：
```typescript
setRoles(realRoles as any);
```
这是因为 `DEFAULT_ROLES` 和 `Role` 类型不兼容。本 Story 统一格式后应消除 `as any`。

#### 陷阱 6：Sidebar.tsx 使用 `any` 类型

当前 Sidebar props 全部是 `any`，应在本 Story 中为角色相关 props 添加类型约束：
```typescript
interface SidebarProps {
  roles: DisplayRole[];
  currentView: string;
  // ... 其他 props 保持 any（后续 Story 逐步类型化）
}
```

#### 陷阱 7：其他组件引用 DEFAULT_ROLES 格式

以下组件也引用角色数据，格式变更可能影响它们：
- `ButlerView.tsx` — 使用 `roles` prop 渲染仪表盘角色卡片（`role.icon`、`role.color`、`role.text`、`role.tint`）
- `RoleView.tsx` — 使用 `role` prop 渲染角色头部
- `DashboardTab.tsx` — 使用 `roles` prop
- `WeeklyReviewModal.tsx` — 使用 `roles` prop

**本 Story 范围：** 只修改 Sidebar 和 RoleSidebarIcon 的渲染方式。其他组件暂时继续使用 `DEFAULT_ROLES` 格式（mock 模式），在后续 Story 中逐步接通。为避免破坏其他组件，`DEFAULT_ROLES` mock 数据可保持原格式不变，Sidebar 部分新增兼容渲染逻辑（同时支持 React 组件和 emoji 字符串格式的 icon）。

**推荐方案（最小破坏）：** 
1. RoleSidebarIcon 内部做 icon 渲染判断：如果 `typeof role.icon === 'string'` → 渲染 emoji 文字；如果是函数/组件 → 渲染 `<role.icon size={22} />`
2. color 渲染也做判断：如果以 `#` 开头 → 用 inline style；否则 → 用 Tailwind class
3. 能量状态小点统一基于 `energy` 值（DEFAULT_ROLES 已有 energy 字段）
4. 这样 mock 和真实数据都能正确渲染，无需改动其他组件

### 前一 Story (1.8) 关键经验

- **SQLx 0.8** 使用运行时查询 `sqlx::query_as::<_, T>(sql)...`，不用 `sqlx::query!` 宏
- **Tauri 2.x async Command** 的 `State` 参数生命周期必须是 `'_`
- **现有 Cargo.toml 依赖**：tauri 2, serde, serde_json, tokio (full), tracing, sqlx 0.8 (sqlite), uuid 1, keyring 3, reqwest 0.12, chrono 0.4
- **`role_list` command 已存在**：`commands/role.rs` 已有 `role_list` → 调 `db::roles::list_active_roles` 返回 `Vec<Role>`
- **`roleService.list()` 前端 service 已存在**：`services/roleService.ts` 已封装 `invoke<Role[]>('role_list')`
- **App.tsx 已有真实角色加载逻辑**：`useEffect` 中调用 `appService.isFirstLaunch()` + `roleService.list()`，非首次启动时加载真实角色数据

### 不需要修改的后端文件

本 Story 是纯前端工作，**不需要任何 Rust 后端修改**：
- `db/roles.rs` — 不变
- `commands/role.rs` — 不变
- `models/role.rs` — 不变
- `lib.rs` — 不变
- `services/` — 不变
- `migrations/` — 不变

### Project Structure Notes

**本 Story 新建文件：**

| 文件 | 内容 |
|---|---|
| `GUI/src/components/layout/RoleSidebarIcon.tsx` | 独立角色侧边栏图标组件 |
| `GUI/src/components/layout/RoleSidebarIcon.test.tsx` | 组件测试 |

**本 Story 修改文件：**

| 文件 | 修改内容 |
|---|---|
| `GUI/src/components/layout/Sidebar.tsx` | 角色渲染改用 RoleSidebarIcon 组件 + 添加语义化属性 + 键盘导航 |
| `GUI/src/index.css` | `.breathe` class 添加 `will-change: opacity` |

**不修改的文件（确认无需改动）：**
- `GUI/src/App.tsx` — 真实角色加载已就绪，无需改动
- `GUI/src/constants/mockData.ts` — 保持原格式，RoleSidebarIcon 兼容两种格式
- `GUI/src/types/role.ts` — 后端 Role 类型不变
- `GUI/src/components/butler/ButlerView.tsx` — 不改，后续 Story 处理
- `GUI/src/components/role/RoleView.tsx` — 不改，后续 Story 处理
- 所有 Rust 后端文件 — 纯前端 Story

**与架构文档对齐：**

| 架构规范要求 | 本 Story 实现 |
|---|---|
| 组件域目录 `components/layout/` | ✅ RoleSidebarIcon 放在 layout/ |
| 侧边栏 `role="navigation"` | ✅ 添加语义化属性 |
| 呼吸动画 CSS keyframe（不引入额外动画库）| ✅ 使用现有 `.breathe` CSS |
| `prefers-reduced-motion` 支持 | ✅ 已有全局规则 |
| 能量值色谱 高翠绿/中琥珀/低暗灰 | ✅ RoleSidebarIcon 基于 energy 计算 |
| WCAG 2.1 AA 键盘导航 | ✅ Tab + 上下键 + Enter |
| aria-label 无障碍 | ✅ 角色名 + 能量值 |
| 前端组件测试 co-located | ✅ `RoleSidebarIcon.test.tsx` 同目录 |

### 类型映射参考

| 数据来源 | icon 格式 | color 格式 | energy | status |
|---|---|---|---|---|
| `DEFAULT_ROLES` (mock) | `LucideIcon` (React 组件) | `'bg-indigo-600'` (Tailwind) | `number` | `'green'\|'yellow'\|'none'` |
| 后端 `Role` (真实) | `string` (emoji) | `string` (hex `#4F46E5`) | `number` | `'active'\|'archived'` |
| `RoleSidebarIcon` props | `string \| React.ComponentType` (兼容) | `string` (两种格式) | `number` | 不使用，基于 energy 计算小点 |

### References

- [Source: `_bmad-output/planning-artifacts/epics.md` #Story 1.9] — AC 定义
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Frontend Architecture] — 组件域目录
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` #RoleSidebarIcon] — 呼吸动画规范（opacity 0.65→1, 3s, ease-in-out）
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` #Accessibility] — WCAG 2.1 AA, 键盘导航, aria-label
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` #Energy 色谱] — 高翠绿/中琥珀/低暗灰
- [Source: `GUI/src/components/layout/Sidebar.tsx`] — 当前侧边栏实现（117行）
- [Source: `GUI/src/index.css`] — 当前 `.breathe` 动画定义
- [Source: `GUI/src/constants/mockData.ts`] — DEFAULT_ROLES mock 格式
- [Source: `GUI/src/types/role.ts`] — 后端 Role TS 类型（emoji icon + hex color）
- [Source: `GUI/src-tauri/src/db/roles.rs`] — 角色 DB 层（icon TEXT, color TEXT）
- [Source: `_bmad-output/implementation-artifacts/1-8-onboarding-five-step-first-role.md`] — 前一 Story 经验

### 验证命令清单

```bash
cd GUI
npx tsc --noEmit               # TypeScript 编译检查
npm run test:frontend           # 前端测试（含新 RoleSidebarIcon 测试）

cd GUI/src-tauri
cargo test                      # 后端测试无回归

cd GUI
npm run tauri dev               # 集成验证：侧边栏图标呼吸动画 + 真实角色数据
```

**手动验证清单：**
- [ ] Chrome DevTools Performance：呼吸动画 60fps 无掉帧
- [ ] Chrome DevTools Rendering → CSS animation → `will-change` 触发 GPU 合成层
- [ ] 系统设置开启 reduce motion → 动画停止
- [ ] 屏幕阅读器（NVDA/VoiceOver）焦点到角色图标 → 朗读角色名和能量值
- [ ] Tab 键聚焦侧边栏 → 上下键切换 → Enter 进入角色视图
- [ ] 后端创建角色（emoji icon）→ 侧边栏正确显示 emoji
- [ ] mock 模式（无后端）→ DEFAULT_ROLES 仍正确显示

## Dev Agent Record

### Implementation Plan

- Phase 1: 创建 RoleSidebarIcon 独立组件（emoji/Lucide 兼容、呼吸动画、能量状态小点、键盘导航、aria-label）
- Phase 2: 重构 Sidebar 使用 RoleSidebarIcon 组件 + 语义化属性 + 键盘导航
- Phase 3: 角色数据桥接（RoleSidebarIcon 内部兼容 mock 和真实两种格式，不修改其他组件）
- Phase 4: 12 个测试用例全部通过

### Debug Log References

### Completion Notes List

- ✅ RoleSidebarIcon 组件：兼容 emoji 字符串和 Lucide React 组件两种 icon 格式
- ✅ RoleSidebarIcon 组件：兼容 hex 色值（inline style）和 Tailwind class 两种 color 格式
- ✅ 能量状态小点：基于 energy 值动态计算（≥80 翠绿 / ≥40 琥珀 / <40 暗灰），不依赖 mock status 字段
- ✅ 呼吸动画：非选中时添加 `breathe` CSS class，`will-change: opacity` 启用 GPU 加速
- ✅ `prefers-reduced-motion` 支持：已有全局规则生效
- ✅ 无障碍：`aria-label` 包含角色名和能量值，focus-visible 环，Enter/Space 键触发
- ✅ 键盘导航：Sidebar 容器监听 ArrowUp/ArrowDown 切换焦点
- ✅ 语义化：`<aside role="navigation" aria-label="角色导航">`
- ✅ 零后端修改：纯前端 Story
- ✅ 未修改 DEFAULT_ROLES 或其他组件，最小化破坏范围

### File List

**新建文件：**
- `GUI/src/components/layout/RoleSidebarIcon.tsx` — 独立角色侧边栏图标组件（87 行）
- `GUI/src/components/layout/RoleSidebarIcon.test.tsx` — 12 个测试用例

**修改文件：**
- `GUI/src/components/layout/Sidebar.tsx` — 角色渲染改用 RoleSidebarIcon + 语义化属性 + 键盘导航
- `GUI/src/index.css` — `.breathe` class 添加 `will-change: opacity`

## Change Log

| 日期 | 变更 |
|------|------|
| 2026-05-22 | Story 1.9 上下文文件创建 — ready-for-dev |
| 2026-05-23 | Story 1.9 实现完成 — review |
| 2026-05-23 | Code Review 修复 3 项：键盘导航选择器、renderIcon 防御、冗余参数 — done |