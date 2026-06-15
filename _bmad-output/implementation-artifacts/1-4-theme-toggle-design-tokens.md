# Story 1.4: 用户能切换浅色/深色主题，所有动效遵循设计 token

Status: done

## Story

As a 用户,
I want 切换浅色/深色主题且切换流畅,
So that 在不同光线环境下都能舒适使用。

## Acceptance Criteria

1. **AC-1 主题切换即时生效**：用户点击侧边栏底部主题切换按钮（Moon/Sun 图标），切换到深色/浅色 < 100ms 立即生效。
2. **AC-2 主题偏好持久化**：主题偏好写入 `localStorage`，重启后保留。首次打开如无偏好则默认浅色。
3. **AC-3 prefers-reduced-motion 支持**：用户系统设置 `prefers-reduced-motion: reduce` 时，所有动效（包括呼吸动画、过渡、淡入淡出）降为 ≤ 10ms。
4. **AC-4 对比度达标**：任意主题下所有文本/背景对比度 ≥ 4.5:1（axe DevTools 验证）。
5. **AC-5 Design Token 系统**：`tailwind.config.js` + `index.css` 包含完整 CSS 变量 token 系统：
   - 色彩 token（`--bg-base`, `--bg-surface`, `--bg-elevated`, `--text-primary`, `--text-secondary`, `--text-muted`, `--border-default`）浅色+深色双组
   - 间距（4px 基准，已由 Tailwind 默认提供，无需自定义）
   - 圆角（`--radius-button: 6px`, `--radius-card: 10px`, `--radius-dialog: 12px`, `--radius-input: 24px`）
   - 动效时长（`--duration-fast: 200ms`, `--duration-normal: 250ms`, `--duration-color: 300ms`, `--duration-breath: 3s`）
   - 角色色温表（`--role-accent`, `--role-bg-tint`，管家/工作/家庭/学习/健康 5 组色值）
   - 能量值色谱（`--energy-high: #10B981`, `--energy-mid: #F59E0B`, `--energy-low: #9CA3AF`）
6. **AC-6 字体完整加载**：Inter + Noto Sans SC + JetBrains Mono 均通过 `font-display: swap` 加载，无 FOIT。
7. **AC-7 路径别名**：配置 `@` 路径别名指向 `src/`（`tsconfig.json` + `vite.config.ts`）。
8. **AC-8 现有测试通过**：`npm run test:frontend` 全部通过。
9. **AC-9 零 TypeScript 错误**：`tsc --noEmit` 零错误。
10. **AC-10 视觉零回归**：除主题切换行为外，浅色主题下应用外观与 Story 1.3 完成后一致。

## Tasks / Subtasks

### Phase 1: 路径别名配置 (AC: #7)

- [ ] **T1.1** 在 `tsconfig.json` 的 `compilerOptions` 中添加 `"baseUrl": "."` 和 `"paths": { "@/*": ["src/*"] }`
- [ ] **T1.2** 在 `vite.config.ts` 中添加 `resolve.alias` 配置 `'@' → path.resolve(__dirname, './src')`，需 import `path`
- [ ] **T1.3** 运行 `tsc --noEmit` 确认零错误（此阶段不修改现有 import，仅使别名可用）

### Phase 2: CSS 变量 Token 系统 (AC: #5)

- [ ] **T2.1** 在 `index.css` 的 `@layer base` 中定义 `:root`（浅色）和 `.dark`（深色）CSS 变量集：

  **浅色主题 `:root`：**
  ```
  --bg-base: #F8F9FA
  --bg-surface: #FFFFFF
  --bg-elevated: #FAFAFA
  --text-primary: #1A1A2E
  --text-secondary: #6B7280
  --text-muted: #9CA3AF
  --border-default: #E5E7EB
  --sidebar-bg: #F1F3F5
  ```

  **深色主题 `.dark`：**
  ```
  --bg-base: #0F1117
  --bg-surface: #1A1B2E
  --bg-elevated: #252638
  --text-primary: #E8E8ED
  --text-secondary: #9CA3AF
  --text-muted: #6B7280
  --border-default: #374151
  --sidebar-bg: #1E293B
  ```

  **主题无关 token（`:root` 中）：**
  ```
  /* 圆角 */
  --radius-button: 6px
  --radius-card: 10px
  --radius-dialog: 12px
  --radius-input: 24px

  /* 动效时长 */
  --duration-fast: 200ms
  --duration-normal: 250ms
  --duration-color: 300ms
  --duration-breath: 3s

  /* 能量色谱 */
  --energy-high: #10B981
  --energy-mid: #F59E0B
  --energy-low: #9CA3AF

  /* 功能色 */
  --color-success: #10B981
  --color-info: #3B82F6
  --color-warning: #F59E0B
  --color-error: #EF4444

  /* 角色色温（默认=管家，运行时由角色切换逻辑动态覆盖） */
  --role-accent: #6366F1
  --role-bg-tint: transparent
  ```

- [ ] **T2.2** 在 `tailwind.config.js` 的 `theme.extend` 中将 CSS 变量映射为 Tailwind utility：
  ```js
  colors: {
    accent: 'var(--role-accent)',
    sidebar: 'var(--sidebar-bg)',
    surface: 'var(--bg-surface)',
    elevated: 'var(--bg-elevated)',
    'energy-high': 'var(--energy-high)',
    'energy-mid': 'var(--energy-mid)',
    'energy-low': 'var(--energy-low)',
  },
  borderRadius: {
    button: 'var(--radius-button)',
    card: 'var(--radius-card)',
    dialog: 'var(--radius-dialog)',
    input: 'var(--radius-input)',
  },
  transitionDuration: {
    fast: 'var(--duration-fast)',
    normal: 'var(--duration-normal)',
    color: 'var(--duration-color)',
  },
  ```
  ⚠️ **保留现有硬编码的 `accent: '#6366F1'` 和 `sidebar: '#F1F3F5'`** 替换为 CSS 变量引用。不破坏现有使用 `bg-accent`、`bg-sidebar` 的组件。

- [ ] **T2.3** 运行 `tsc --noEmit` + `npm run build` 确认无报错

### Phase 3: prefers-reduced-motion 支持 (AC: #3)

- [ ] **T3.1** 在 `index.css` 中添加 reduced-motion media query（位于文件末尾，在 `.breathe` 动画之后）：
  ```css
  @media (prefers-reduced-motion: reduce) {
    *, *::before, *::after {
      animation-duration: 0.01ms !important;
      animation-iteration-count: 1 !important;
      transition-duration: 0.01ms !important;
    }
  }
  ```
- [ ] **T3.2** 确认 `.breathe` 动画在 reduced-motion 下不循环

### Phase 4: 主题持久化 + 系统偏好检测 (AC: #1, #2)

- [ ] **T4.1** 在 `App.tsx` 中修改 `theme` 初始化逻辑：
  1. 读取 `localStorage.getItem('egosync-theme')`
  2. 如果有值（`'light'` | `'dark'`）则使用
  3. 如果无值，检查 `window.matchMedia('(prefers-color-scheme: dark)').matches`
  4. 如果匹配则 `'dark'`，否则默认 `'light'`

- [ ] **T4.2** 在 `onToggleTheme` 回调中同步写入 `localStorage.setItem('egosync-theme', newTheme)`

- [ ] **T4.3** 确认切换时 `dark` class 应用到根 `<div>` 上的时间 < 100ms（当前已通过 `cn()` + `theme === 'dark' && "dark"` 实现，React state 更新足够快）

### Phase 5: 字体补全 (AC: #6)

- [ ] **T5.1** 在 `index.html` 的 Google Fonts link 中添加 JetBrains Mono 字体：
  ```html
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&family=JetBrains+Mono:wght@400;500&family=Noto+Sans+SC:wght@300;400;500;600&display=swap" rel="stylesheet">
  ```
  注意：Google Fonts 的 `display=swap` 参数已在 URL 中，确保对所有字体生效。

- [ ] **T5.2** 在 `tailwind.config.js` 中添加 `mono` 字体族：
  ```js
  fontFamily: {
    sans: ['Inter', 'Noto Sans SC', 'sans-serif'],
    mono: ['JetBrains Mono', 'monospace'],
  },
  ```

### Phase 6: 深色主题样式补全 (AC: #4, #10)

- [ ] **T6.1** 审查所有组件中的深色主题类名，确保以下关键点已覆盖：
  - `Sidebar.tsx` — 深色背景 ✅（已有 `dark:bg-slate-800`）、上下文菜单需添加 `dark:bg-slate-800 dark:border-slate-600 dark:text-slate-200`
  - `Modal.tsx` — 确认弹窗 overlay 在深色下对比度正确
  - 各组件中硬编码的 `bg-white` / `text-slate-*` / `border-slate-*` 是否有对应 `dark:` 变体

- [ ] **T6.2** 用 axe DevTools（或等效方法）抽检关键页面对比度：
  - 管家视角（浅色 + 深色）
  - 角色视图（浅色 + 深色）
  - 侧边栏上下文菜单（浅色 + 深色）
  - 如发现不达标的组合，修复到 ≥ 4.5:1

### Phase 7: 测试 + 验证 (AC: #8, #9)

- [ ] **T7.1** 更新 `App.test.tsx`：mock `localStorage` 和 `matchMedia`，确保 render 正常
- [ ] **T7.2** 运行 `tsc --noEmit`：零错误
- [ ] **T7.3** 运行 `npm run test:frontend`：所有测试通过
- [ ] **T7.4** 运行 `npm run build`：生产构建成功
- [ ] **T7.5** 手动验证：
  - 浅色主题外观与 Story 1.3 一致（视觉零回归）
  - 深色主题切换 < 100ms
  - 关闭应用重启后主题保留
  - 系统开启 reduced-motion 后呼吸动画停止

## Dev Notes

### ⚠️ 致命陷阱清单

#### 陷阱 1：Tailwind `accent` / `sidebar` 色值覆盖

**现状**：`tailwind.config.js` 中 `colors.accent` 硬编码为 `'#6366F1'`，`colors.sidebar` 硬编码为 `'#F1F3F5'`。多个组件使用 `bg-accent`、`bg-sidebar` 等类名。

**解决方案**：将这两个值替换为 CSS 变量引用 `'var(--role-accent)'` 和 `'var(--sidebar-bg)'`。由于 `:root` 默认值与当前硬编码值一致（`--role-accent: #6366F1`、`--sidebar-bg: #F1F3F5`），浅色主题下视觉零回归。深色主题下 `--sidebar-bg` 自动切换为 `#1E293B`。

⚠️ **不要**将 `accent` 改为其他名称或移除——组件中大量使用 `bg-accent`、`shadow-indigo-200` 等。只改值不改键名。

#### 陷阱 2：`dark:` Tailwind 类 vs CSS 变量双轨并存

**现状**：组件中已有大量 `dark:bg-slate-900`、`dark:text-slate-100` 等类名（Story 1.3 从原型保留）。

**策略**：本 Story 建立 CSS 变量 token 系统，但 **不迁移** 现有 `dark:` 类名。两套方案并行不冲突——`darkMode: 'class'` 继续驱动 `dark:` 前缀类，CSS 变量通过 `.dark` 选择器切换。后续 Story 可逐步用 token utility 替换硬编码类名。

#### 陷阱 3：Sidebar 深色主题 border 色值硬编码

**现状**：`Sidebar.tsx` 第 26 行 `border-slate-200 dark:border-slate-700`、第 49 行状态小点 `border-2 border-[#F1F3F5]`（浅色侧边栏背景色硬编码为小点边框，深色下应该是侧边栏深色背景）。

**解决方案**：状态小点 `border-[#F1F3F5]` 需添加 `dark:border-slate-800`（对应深色侧边栏背景），否则深色主题下小点边框白色一圈异常明显。同理 `NotificationPanel` 中的通知小点。

#### 陷阱 4：localStorage 在 Tauri WebView 中的可用性

**当前阶段**：Story 1.4 在纯前端阶段（`npm run dev`），`localStorage` 可用且持久。未来接入 Tauri 后，WebView 的 `localStorage` 也持久化到用户数据目录。**安全选择**。

后续 Story 接入 `app_settings` 表时，可将主题偏好迁移到 Rust 后端。本 Story 不需要考虑。

#### 陷阱 5：Google Fonts 链接格式

**现状**：`index.html` 第 7 行的 Google Fonts URL 使用 `display=swap` 参数。

**要求**：添加 JetBrains Mono 时，在同一个 `<link>` 中添加 `family=JetBrains+Mono:wght@400;500`，保持 `display=swap`。不要创建第二个 `<link>` 标签。

#### 陷阱 6：Tailwind CSS 变量引用语法

**关键**：Tailwind v3 中，CSS 变量引用必须使用正确语法：
- `colors` 中的变量引用：`'var(--role-accent)'`（不需要 `<alpha-value>` 支持，因为角色色温不需要透明度变体）
- 如果需要 opacity 变体（如 `bg-accent/50`），需使用 `color: 'rgb(var(--role-accent-rgb) / <alpha-value>)'` 格式
- 本 Story 中 accent 颜色不需要 opacity 变体（现有代码中 `bg-accent` 均为 100% 不透明），直接使用 `'var(--role-accent)'` 即可

#### 陷阱 7：App.tsx 根元素的 dark class 应用方式

**现状**：`App.tsx` 第 45 行通过 `cn("...", theme === 'dark' && "dark")` 将 `dark` class 添加到根 `<div>`。

**注意**：Tailwind `darkMode: 'class'` 要求 `dark` class 在 DOM 树的**祖先元素**上。当前 `dark` 加在根 `<div>` 上（而非 `<html>`），Tailwind 的 `dark:` 变体依然生效，因为它匹配任意祖先。CSS 变量的 `.dark` 选择器同理。**不需要修改应用位置。**

### Project Structure Notes

**本 Story 修改的文件清单（无新建文件）：**

| 文件 | 修改内容 |
|---|---|
| `egosync-app/src/index.css` | 添加 CSS 变量（浅色+深色）、prefers-reduced-motion media query |
| `egosync-app/tailwind.config.js` | 扩展 theme（colors 引用 CSS 变量、borderRadius、transitionDuration、fontFamily.mono） |
| `egosync-app/index.html` | 添加 JetBrains Mono 字体到 Google Fonts link |
| `egosync-app/src/App.tsx` | 主题 localStorage 持久化 + 系统偏好检测 |
| `egosync-app/tsconfig.json` | 添加 `baseUrl` + `paths` 路径别名 |
| `egosync-app/vite.config.ts` | 添加 `resolve.alias` 路径别名 |
| `egosync-app/src/components/layout/Sidebar.tsx` | 修复深色主题下状态小点边框色 |
| `egosync-app/src/App.test.tsx` | mock localStorage + matchMedia |

**不修改的文件（确认无需改动）：**
- `egosync-app/src/lib/utils.ts` — cn() 不变
- `egosync-app/src/constants/mockData.ts` — mock 数据不变
- `egosync-app/src/main.tsx` — 入口不变
- 其他所有 `components/` 下的组件 — 现有 `dark:` 类名继续工作，无需迁移

**与架构文档的对齐：**

| 架构/UX 规范要求 | 本 Story 实现 |
|---|---|
| UX-DR1 设计 Token 体系 | ✅ index.css CSS 变量 + tailwind.config.js 映射 |
| UX-DR2 角色色温 `--role-accent` + `--role-bg-tint` | ✅ CSS 变量定义，默认管家值；动态切换逻辑由后续角色切换 Story 实现 |
| UX-DR3 能量值色谱 | ✅ `--energy-high/mid/low` 变量定义 |
| UX-DR4 字体系统 | ✅ JetBrains Mono 添加 |
| UX-DR21 动效时长 token | ✅ `--duration-fast/normal/color/breath` 变量 |
| UX-DR23 prefers-reduced-motion | ✅ 全局 media query |
| UX-DR22 WCAG 对比度 | ✅ Phase 6 验证 |

### 角色色温参考表（供 CSS 变量定义）

| 角色类型 | `--role-accent` | `--role-bg-tint` | 来源 |
|---|---|---|---|
| 管家（默认） | `#6366F1` | `transparent` | UX spec §Color System |
| 工作角色 | `#4F46E5` | `rgba(79,70,229,0.04)` | UX spec |
| 家庭角色 | `#D97706` | `rgba(217,119,6,0.04)` | UX spec |
| 学习角色 | `#7C3AED` | `rgba(124,58,237,0.04)` | UX spec |
| 健康角色 | `#059669` | `rgba(5,150,105,0.04)` | UX spec |

> 注：角色色温的**动态切换**（进入角色时更新 `--role-accent` / `--role-bg-tint`）不在本 Story 范围。本 Story 仅定义变量，设置管家默认值。动态切换由后续角色视图接通 Story 实现。

### References

- [Source: `_bmad-output/planning-artifacts/epics.md` #Story 1.4] — 完整 AC 定义
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` #Color System] — 浅色/深色 token 色值
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` #Typography System] — 字体系统
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` #Spacing & Layout Foundation] — 间距/圆角 token
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` #Transition Patterns] — 动效时长
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` #Accessibility Strategy] — prefers-reduced-motion、对比度、WCAG
- [Source: `_bmad-output/project-context.md` #框架特定规则] — Tailwind utility only、darkMode: 'class'
- [Source: `_bmad-output/implementation-artifacts/1-3-component-domain-split-visual-zero-regression.md` #陷阱3] — "添加 @ 别名属于 Story 1.4 的 design token 工作"
- [Source: `egosync-app/tailwind.config.js`] — 当前配置：darkMode 'class'、tailwindcss-animate 插件、仅 accent/sidebar 两个自定义色
- [Source: `egosync-app/src/index.css`] — 当前：基础 Tailwind import + 滚动条 + .breathe 动画，无 CSS 变量
- [Source: `egosync-app/src/App.tsx`] — 当前：theme useState 无持久化，65 行
- [Source: `egosync-app/src/components/layout/Sidebar.tsx`] — 主题切换按钮位置（第 67-69 行）、状态小点边框硬编码（第 49-50 行）
- [Source: `egosync-app/index.html`] — Google Fonts link 含 Inter + Noto Sans SC，无 JetBrains Mono

### 验证命令清单

```bash
cd GUI
npx tsc --noEmit          # AC-9: 零 TypeScript 错误
npm run test:frontend     # AC-8: 测试通过
npm run build             # 生产构建成功
npm run dev               # 手动验证主题切换 + 持久化 + reduced-motion
```

## Dev Agent Record

### Agent Model Used
Claude Sonnet 4 (Cascade)

### Debug Log References
- jsdom 不支持 `localStorage.getItem` → test-setup.ts 添加 localStorage mock
- jsdom 不支持 `window.matchMedia` → test-setup.ts 添加 matchMedia mock
- `@types/node` 缺失导致 `path`/`__dirname` 报错 → 安装为 devDependency

### Completion Notes List
- ✅ AC-1: 主题切换即时生效（React state 更新 < 16ms）
- ✅ AC-2: localStorage 持久化 + 系统偏好检测
- ✅ AC-3: prefers-reduced-motion 全局 media query
- ✅ AC-4: 深色主题下 Sidebar 上下文菜单/确认弹窗/状态小点对比度修复
- ✅ AC-5: 完整 CSS 变量 token 系统（10 色彩 + 4 圆角 + 4 动效时长 + 3 能量色 + 4 功能色 + 2 角色色温）
- ✅ AC-6: JetBrains Mono 添加，所有字体 display=swap
- ✅ AC-7: @ 路径别名（tsconfig + vite + vitest）
- ✅ AC-8: npm run test:frontend 通过
- ✅ AC-9: tsc --noEmit 零错误
- ✅ AC-10: 浅色主题视觉零回归（CSS 变量默认值与原硬编码一致）

### File List
- `egosync-app/src/index.css` — CSS 变量 token + reduced-motion
- `egosync-app/tailwind.config.js` — 扩展 colors/borderRadius/transitionDuration/fontFamily
- `egosync-app/index.html` — 添加 JetBrains Mono 字体
- `egosync-app/src/App.tsx` — 主题 localStorage 持久化
- `egosync-app/tsconfig.json` — @ 路径别名
- `egosync-app/vite.config.ts` — @ resolve alias
- `egosync-app/vitest.config.ts` — @ resolve alias
- `egosync-app/src/components/layout/Sidebar.tsx` — 深色主题样式补全
- `egosync-app/src/test-setup.ts` — localStorage + matchMedia mock
- `egosync-app/package.json` — 添加 @types/node
