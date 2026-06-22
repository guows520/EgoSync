# Story 4.5: 三级通知系统（耳语/轻触/敲门）

Status: done

## Story

As a 用户,
I want 收到不同紧急程度的通知且不被过度打扰,
so that 重要信息不遗漏同时保持专注。

## Acceptance Criteria

1. **AC-1 耳语级通知（Whisper）**：角色生成"耳语"级通知 → 静默积累到通知列表，不显示任何前台提示，在下次晨间简报中汇总（晨间简报属 Epic 6，本 Story 只需写入 DB + 通知列表可查）

2. **AC-2 轻触级通知（Tap）**：角色生成"轻触"级通知 → 侧边栏通知铃铛显示红点 badge，不弹窗打断用户当前操作

3. **AC-3 敲门级通知（Knock）**：角色生成"敲门"级通知 → 管家对话区嵌入 ActionCard 展示通知（图标 + 标题 + 来源角色 + 时间 + "立即处理 / 稍后"按钮），侧边栏铃铛显示红点 badge，可选声音提示（用户在设置中开启，默认关闭），遵守 UX-DR16（不使用传统 toast/snackbar）+ UX-DR20（三级通知视觉规则）

4. **AC-4 敲门降级**：同一天已有 3 次"敲门"通知，第 4 次"敲门"触发时 → 自动降级为"轻触"，tracing 日志记录降级

5. **AC-5 通知面板**：用户点击通知铃铛 → NotificationPanel 打开，按时间倒序显示所有通知，每条显示：角色色标 + 角色名 + 级别标签（耳语/轻触/敲门）+ 内容 + 时间，敲门级通知标签带 `animate-pulse`

6. **AC-6 数据库**：`migrations/017_notifications.sql` 创建 `notifications` 表：`id`, `role_id`, `level`(whisper/tap/knock), `content`, `is_read`, `created_at`

7. **AC-7 Rust 后端命令**：Tauri commands: `notification::create` / `notification::list` / `notification::mark_read`，Tauri Event: `notification:new { level, content, role_name }` 供前端实时响应

8. **AC-8 前端接通**：NotificationPanel 接通真实数据（替换原型 mock 列表），`useNotifications()` hook + `useTauriEvent('notification:new')` 实时更新

9. **AC-9 已读标记**：通知面板中点击通知条目 → `is_read` 更新为 true，红点 badge 根据未读数显示/隐藏

10. **AC-10 通知级别上限约束**：通知创建时根据角色 `proactivity_level` 约束最高通知级别（复用 `max_notification_level_for_proactivity`），超出上限的级别自动降级到允许的最高级别

## Tasks / Subtasks

- [x] Task 1: 数据库迁移 (AC: #6)
  - [x] 1.1 创建 `migrations/017_notifications.sql`
  - [x] 1.2 在 `db/mod.rs` 注册 `notifications` 模块
  - [x] 1.3 在 `models/mod.rs` 注册 `notification` 模块

- [x] Task 2: Rust 数据模型 (AC: #6, #7)
  - [x] 2.1 创建 `models/notification.rs`：`Notification` + `NotificationWithRole` + `CreateNotificationInput` + `NotificationLevel` 枚举
  - [x] 2.2 所有 struct 使用 `#[serde(rename_all = "camelCase")]` + `sqlx::FromRow`（参考 `models/suggestion.rs`）

- [x] Task 3: Rust DB 层 (AC: #6, #7)
  - [x] 3.1 创建 `db/notifications.rs`：`create_notification` / `list_notifications` / `mark_read` / `count_unread` / `count_knock_today`
  - [x] 3.2 `create_notification` 内部调用降级逻辑：先检查 `count_knock_today`，若 >= 3 且 level=knock 则降级为 tap + tracing 日志
  - [x] 3.3 `create_notification` 内部调用 `max_notification_level_for_proactivity` 约束级别上限

- [x] Task 4: Rust Tauri 命令 (AC: #7)
  - [x] 4.1 创建 `commands/notification.rs`：`notification_create` / `notification_list` / `notification_mark_read`
  - [x] 4.2 `notification_create` 写入 DB 后 emit Tauri Event `notification:new`（payload: `{ level, content, roleName, roleId, roleIcon, roleColor, id, createdAt }`）
  - [x] 4.3 在 `commands/mod.rs` 注册 `notification` 模块
  - [x] 4.4 在 `lib.rs` 的 `invoke_handler` 注册 3 个命令

- [x] Task 5: Rust 通知创建服务集成 (AC: #4, #10)
  - [x] 5.1 在 `services/mod.rs` 注册 `notification_service` 模块
  - [x] 5.2 创建 `services/notification_service.rs`：`create_notification_for_role` 函数，接收 `pool, role_id, level, content`，内部读取角色 `proactivity_level`，调用 `max_notification_level_for_proactivity` 约束 + `count_knock_today` 降级 + 写入 DB + 返回最终级别
  - [x] 5.3 在 `scheduler.rs` 的 `run_work_loop_for_role` 中，建议写入成功后调用 `notification_service::create_notification_for_role` 为每条建议生成对应级别的通知（high→knock, medium→tap, low→whisper）

- [x] Task 6: 前端类型定义 (AC: #8)
  - [x] 6.1 创建 `types/notification.ts`：`Notification` + `NotificationWithRole` 接口，字段 camelCase 对齐 Rust serde

- [x] Task 7: 前端 Service 层 (AC: #8)
  - [x] 7.1 创建 `services/notificationService.ts`：`list` / `create` / `markRead` / `countUnread`，封装 `invoke` 调用

- [x] Task 8: 前端 Hook (AC: #8, #9)
  - [x] 8.1 创建 `hooks/useNotifications.ts`：加载通知列表 + `useTauriEvent('notification:new')` 实时追加 + `markAsRead` + `unreadCount` 派生值
  - [x] 8.2 事件 payload 到达时自动追加到列表头部，并更新未读计数

- [x] Task 9: 前端 NotificationPanel 接通真实数据 (AC: #5, #8, #9)
  - [x] 9.1 修改 `NotificationPanel.tsx`：移除 `MOCK_NOTIFICATIONS` 导入，使用 `useNotifications()` hook
  - [x] 9.2 每条通知显示：角色色标 + 角色名 + 级别标签 + 内容 + 相对时间
  - [x] 9.3 敲门级通知标签保留 `animate-pulse`
  - [x] 9.4 点击通知条目调用 `markAsRead`，已读通知视觉降低 opacity
  - [x] 9.5 空状态显示有温度文案："暂时没有新通知"

- [x] Task 10: 侧边栏铃铛红点动态化 (AC: #2, #3, #9)
  - [x] 10.1 在 `App.tsx` 中使用 `useNotifications()` 获取 `unreadCount`
  - [x] 10.2 将 `unreadCount` 传入 `Sidebar` 组件
  - [x] 10.3 修改 `Sidebar.tsx`：红点 badge 根据 `unreadCount > 0` 条件显示（替换当前硬编码红点）
  - [x] 10.4 红点 badge 添加 `aria-label="有新通知"` 无障碍标签

- [x] Task 11: 敲门级通知嵌入管家对话区 (AC: #3)
  - [x] 11.1 在 `ButlerView.tsx` 中监听 `notification:new` 事件，当 level=knock 时将通知转为 ActionCard 数据格式
  - [x] 11.2 复用现有 `ActionCard.tsx` 组件展示敲门通知（"立即处理"= 确认按钮语义，"稍后"= 拒绝按钮语义但 reason="later"）
  - [x] 11.3 敲门通知的 ActionCard 操作完成后调用 `notification_mark_read`

- [x] Task 12: 声音提示设置 (AC: #3)
  - [x] 12.1 在 `app_settings` 中新增 key `notification.knock_sound`（默认 "false"）
  - [x] 12.2 敲门通知到达时读取该设置，若开启则播放简短提示音（使用 Web Audio API，无额外依赖）
  - [x] 12.3 在全局设置 Modal 中新增"敲门通知声音"开关（复用现有设置 UI 模式）

- [x] Task 13: 单元测试 (AC: all)
  - [x] 13.1 Rust: `db/notifications.rs` 测试 — create/list/mark_read/count_unread/count_knock_today
  - [x] 13.2 Rust: `services/notification_service.rs` 测试 — 降级逻辑（4th knock → tap）、proactivity 约束
  - [x] 13.3 Rust: `commands/notification.rs` 测试 — 命令调用链
  - [x] 13.4 前端: `useNotifications.test.ts` — 加载/实时追加/markRead/未读计数

- [x] Task 14: 更新 sprint-status.yaml
  - [x] 14.1 将 `4-5-three-tier-notification` 状态更新为 `done`

## Dev Notes

### 项目背景

本 Story 属于 Epic 4（主动循环、通知与仪表盘），是三级通知系统的核心实现。Story 4.1-4.4 已完成，调度器、建议生成、主动性档位、建议确认/拒绝均已就绪。本 Story 将通知系统从 mock 数据升级为真实数据驱动，并建立完整的三级通知分级机制。

### 技术栈

- **后端**: Rust + Tauri 2.x + SQLx (SQLite) + tokio
- **前端**: React 18 + TypeScript 5.2 + TailwindCSS + Vite 5
- **IPC**: Tauri Commands (invoke) + Tauri Events (emit/listen)

### 关键架构约束

**Rust 三层架构（严格遵守）**：
- `commands/notification.rs` — Tauri 命令接口，薄层，只做参数校验 + 调用 db/services
- `db/notifications.rs` — 纯数据库操作，返回 `Result<T, AppError>`
- `services/notification_service.rs` — 业务逻辑（降级、约束、事件 emit）
- `models/notification.rs` — 数据结构定义

**IPC 命令命名规范**: `notification_create` / `notification_list` / `notification_mark_read`（下划线分隔，参考 `suggestion_list_pending` / `suggestion_confirm`）

**Event 命名规范**: `notification:new`（参考 `llm:stream` 模式，域:动词过去式）

**serde 序列化**: 所有 Rust struct 使用 `#[serde(rename_all = "camelCase")]`，前端 TypeScript 接口字段使用 camelCase

### 已有代码复用（关键 — 避免重复造轮子）

**1. `NotificationLevel` 枚举已定义**：`services/suggestion_generator.rs:354-360` 已定义 `NotificationLevel` 枚举（Whisper/Tap/Knock）和 `max_notification_level_for_proactivity` 函数（line 369-375）。**直接复用，不要重新定义**。

```rust
// services/suggestion_generator.rs:354-375 — 已存在，直接 import 使用
pub enum NotificationLevel { Whisper, Tap, Knock }
pub fn max_notification_level_for_proactivity(proactivity_level: &str) -> NotificationLevel
```

**2. `ActionCard.tsx` 已完整实现**：`components/butler/ActionCard.tsx`（243行）已实现完整的建议卡片组件，支持 confirm/reject/dismiss 动画。敲门通知复用此组件，传入通知数据即可。

**3. `NotificationPanel.tsx` 已有原型**：`components/notifications/NotificationPanel.tsx`（41行）使用 `MOCK_NOTIFICATIONS` 渲染。需替换 mock 数据为 `useNotifications()` hook，保留现有样式和布局。

**4. `useTauriEvent` hook 已实现**：`hooks/useTauriEvent.ts`（32行）提供 Tauri Event 监听能力，直接使用。

**5. `Sidebar.tsx` 红点 badge 已硬编码**：`components/layout/Sidebar.tsx:97` 红点 badge 当前始终显示。改为根据 `unreadCount > 0` 条件渲染。

**6. `MOCK_NOTIFICATIONS` 定义在** `constants/mockData.ts:26-31`，替换后可保留文件中的其他 mock 常量。

**7. `AppError` 枚举**：`error.rs` 已定义 NotFound/DbError/ValidationError 等变体，通知相关错误复用这些变体，不新增。

**8. `db/settings.rs` 的 `chrono_now_pub()`**：时间戳生成函数，创建通知时复用。

**9. `db/app_settings.rs` 的 `get_setting`/`set_setting`**：声音提示设置存储复用此模块。

### 通知级别与建议优先级的映射

调度器 `run_work_loop_for_role` 中建议写入成功后，为每条建议生成通知：
- `priority = "high"` → `level = knock`
- `priority = "medium"` → `level = tap`
- `priority = "low"` → `level = whisper`

### 降级逻辑（两层）

**第一层 — proactivity 约束**：调用 `max_notification_level_for_proactivity(role.proactivity_level)`，若通知级别超出上限则降级到允许的最高级别：
- `proactive` → 允许 Knock
- `moderate` → 最高 Tap（Knock 降级为 Tap）
- `passive` → 最高 Whisper（Knock/Tap 降级为 Whisper）

**第二层 — 每日敲门上限**：查询当天（本地日期）已创建的 `level = 'knock'` 通知数量，若 >= 3 则降级为 Tap，并 `tracing::info!` 记录降级。

降级顺序：先 proactivity 约束，再每日上限约束。

### 数据库 Schema

```sql
-- migrations/017_notifications.sql
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY NOT NULL,
    role_id TEXT NOT NULL,
    level TEXT NOT NULL CHECK (level IN ('whisper', 'tap', 'knock')),
    content TEXT NOT NULL,
    is_read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_notifications_role_id ON notifications(role_id);
CREATE INDEX IF NOT EXISTS idx_notifications_is_read ON notifications(is_read);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at);
CREATE INDEX IF NOT EXISTS idx_notifications_level ON notifications(level);
```

**注意**：迁移文件编号为 `017`（当前最大为 `016_suggestions.sql`）。`is_read` 用 INTEGER（0/1），SQLite 无原生 BOOL 类型。`created_at` 使用 `strftime` 默认值，与 `suggestions` 表保持一致。

### Rust 数据模型参考

参考 `models/suggestion.rs` 的结构模式：

```rust
// models/notification.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub role_id: String,
    pub level: String,
    pub content: String,
    pub is_read: bool,  // sqlx 自动将 INTEGER 0/1 映射为 bool
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotificationInput {
    pub role_id: String,
    pub level: String,
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NotificationWithRole {
    // ... Notification 所有字段
    pub role_name: String,
    pub role_icon: String,
    pub role_color: String,
}
```

### Tauri Event Payload

`notification:new` 事件 payload 结构（camelCase）：

```typescript
interface NotificationNewPayload {
  id: string;
  level: 'whisper' | 'tap' | 'knock';
  content: string;
  roleId: string;
  roleName: string;
  roleIcon: string;
  roleColor: string;
  createdAt: string;
}
```

### 前端类型定义

```typescript
// types/notification.ts
export interface Notification {
  id: string;
  roleId: string;
  level: 'whisper' | 'tap' | 'knock';
  content: string;
  isRead: boolean;
  createdAt: string;
}

export interface NotificationWithRole extends Notification {
  roleName: string;
  roleIcon: string;
  roleColor: string;
}
```

### 前端 Hook 设计

```typescript
// hooks/useNotifications.ts
export function useNotifications() {
  // 加载通知列表
  // useTauriEvent('notification:new') 实时追加
  // markAsRead(id) — 调用 notificationService.markRead + 本地更新 isRead
  // unreadCount — 派生值：notifications.filter(n => !n.isRead).length
  // 返回: { notifications, unreadCount, markAsRead, isLoading, error }
}
```

### UX 设计约束（必须遵守）

**UX-DR16**: 所有反馈通过管家自然语言传达，**不使用传统 toast/snackbar**。敲门通知通过 ActionCard 嵌入管家对话区，不使用弹窗。

**UX-DR20**: 三级通知视觉规则：
- 耳语（L1）→ 侧边栏图标绿点（不打断）
- 轻触（L2）→ 管家摘要中自然语言提及 + 侧边栏铃铛红点
- 敲门（L3）→ ActionCard 出现在管家视角 + 侧边栏铃铛红点

**绝对不使用系统级弹窗/banner/红色 badge 数字。** 所有通知在 EgoSync 的"世界观"内传达。

**动效约束**：
- 敲门级通知标签带 `animate-pulse`（已有原型）
- 所有动效遵守 `prefers-reduced-motion`（`motion-reduce:` 前缀）
- 卡片 hover：`translateY(-1px)` + 阴影加深，200ms

**无障碍**：
- 通知铃铛：`aria-label="有新通知"`（当有未读时）
- NotificationPanel：`role="complementary"`, `aria-label="通知中心"`
- 通知条目：`role="article"`, `aria-label="[角色名] - [级别] 通知"`

### 声音提示实现

不引入额外依赖，使用 Web Audio API：

```typescript
function playNotificationSound() {
  const ctx = new AudioContext();
  const oscillator = ctx.createOscillator();
  const gainNode = ctx.createGain();
  oscillator.connect(gainNode);
  gainNode.connect(ctx.destination);
  oscillator.frequency.value = 880; // A5
  gainNode.gain.setValueAtTime(0.1, ctx.currentTime);
  gainNode.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.3);
  oscillator.start(ctx.currentTime);
  oscillator.stop(ctx.currentTime + 0.3);
}
```

### 调度器集成点

在 `services/scheduler.rs` 的 `run_work_loop_for_role` 函数中（line 182-205），建议写入成功后追加通知创建调用：

```rust
// 在 db::suggestions::create_suggestion 成功后
let notification_level = match s.priority.as_str() {
    "high" => NotificationLevel::Knock,
    "medium" => NotificationLevel::Tap,
    _ => NotificationLevel::Whisper,
};

match notification_service::create_notification_for_role(
    pool, &role.id, notification_level, &s.title
).await {
    Ok(actual_level) => tracing::info!(
        role_id = %role.id, suggestion_id = %s.id,
        requested = ?notification_level, actual = ?actual_level,
        "通知已创建"
    ),
    Err(e) => tracing::warn!(
        role_id = %role.id, error = %e,
        "通知创建失败（不影响建议写入）"
    ),
}
```

**注意**：`create_notification_for_role` 需要接收 `AppHandle` 以便 emit Tauri Event。调度器的 `tokio::spawn` 闭包需要传入 `app_handle`。检查 `spawn_scheduler` 函数签名是否需要修改以传入 `AppHandle`。

### 禁止事项

- **禁止** 前端直接操作 SQLite
- **禁止** 使用 `unwrap()` / `expect()` 处理可能失败的逻辑（测试代码除外）
- **禁止** 硬编码 API Key
- **禁止** 自定义 CSS 文件（只使用 Tailwind utility classes）
- **禁止** 使用 toast/snackbar 组件
- **禁止** 使用系统级弹窗/notification API
- **禁止** 引入额外动画库（CSS transition + keyframe 足够）
- **禁止** 重新定义 `NotificationLevel` 枚举（复用 `suggestion_generator.rs` 中的定义）

### 代码风格

- Rust：`tracing::warn!` 记录非致命错误，`tracing::info!` 记录关键操作
- Rust：所有 `Result` 返回 `AppError`，不使用 `String` 错误
- TypeScript：使用 `interface` 而非 `type` 定义数据结构
- TypeScript：service 层函数返回 `Promise<T>`，不使用 `async` 标记 service 对象方法
- 测试：Rust 使用 `#[tokio::test]` + `sqlite::memory:` 内存数据库（参考 `db/suggestions.rs` 测试模式）

### Project Structure Notes

新增文件：
- `src-tauri/migrations/017_notifications.sql`
- `src-tauri/src/models/notification.rs`
- `src-tauri/src/db/notifications.rs`
- `src-tauri/src/commands/notification.rs`
- `src-tauri/src/services/notification_service.rs`
- `src/types/notification.ts`
- `src/services/notificationService.ts`
- `src/hooks/useNotifications.ts`

修改文件：
- `src-tauri/src/db/mod.rs` — 注册 `notifications` 模块
- `src-tauri/src/models/mod.rs` — 注册 `notification` 模块
- `src-tauri/src/commands/mod.rs` — 注册 `notification` 模块
- `src-tauri/src/services/mod.rs` — 注册 `notification_service` 模块
- `src-tauri/src/lib.rs` — 注册 3 个 Tauri 命令
- `src-tauri/src/services/scheduler.rs` — `run_work_loop_for_role` 中集成通知创建
- `src/components/notifications/NotificationPanel.tsx` — 替换 mock 数据
- `src/components/layout/Sidebar.tsx` — 红点 badge 动态化
- `src/App.tsx` — 使用 `useNotifications` 获取未读数，传入 Sidebar
- `src/components/butler/ButlerView.tsx` — 监听敲门通知，嵌入 ActionCard
- `src/constants/mockData.ts` — 可保留但 `MOCK_NOTIFICATIONS` 不再被引用

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.5] — Story 需求和 AC
- [Source: _bmad-output/planning-artifacts/epics.md#UX-DR16] — 反馈模式约束（不使用 toast/snackbar）
- [Source: _bmad-output/planning-artifacts/epics.md#UX-DR20] — 三级通知视觉规则
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Notification-Levels] — 通知级别定义（L1/L2/L3）
- [Source: _bmad-output/planning-artifacts/architecture.md#核心表设计] — notifications 表 schema
- [Source: _bmad-output/planning-artifacts/architecture.md#IPC命令组织] — 命令命名规范
- [Source: _bmad-output/planning-artifacts/architecture.md#状态同步] — 事件驱动模式
- [Source: _bmad-output/implementation-artifacts/4-4-suggestion-actioncard-confirm-reject.md] — 前一个 Story 的实现模式
- [Source: egosync-app/src-tauri/src/services/suggestion_generator.rs:354-375] — `NotificationLevel` 枚举和 `max_notification_level_for_proactivity` 函数
- [Source: egosync-app/src-tauri/src/commands/suggestion.rs] — Tauri 命令实现模式参考
- [Source: egosync-app/src-tauri/src/db/suggestions.rs] — DB 层实现模式参考
- [Source: egosync-app/src-tauri/src/models/suggestion.rs] — 数据模型模式参考
- [Source: egosync-app/src/components/notifications/NotificationPanel.tsx] — 现有原型组件
- [Source: egosync-app/src/components/butler/ActionCard.tsx] — 敲门通知复用组件
- [Source: egosync-app/src/hooks/useSuggestions.ts] — Hook 实现模式参考
- [Source: egosync-app/src/hooks/useTauriEvent.ts] — Tauri Event 监听 hook
- [Source: egosync-app/src/services/suggestionService.ts] — Service 层实现模式参考
- [Source: egosync-app/src-tauri/src/services/scheduler.rs:144-216] — `run_work_loop_for_role` 集成点
- [Source: egosync-app/src-tauri/src/lib.rs:278-352] — Tauri 命令注册位置
- [Source: egosync-app/src/components/layout/Sidebar.tsx:95-98] — 红点 badge 硬编码位置
- [Source: egosync-app/src/App.tsx:50,315] — 通知面板开关状态和渲染位置

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

### Review Findings

_代码审查 (2026-06-22) — 全部未提交改动 vs 本规格。3 视角（Blind / Edge / Acceptance）。_

- [x] [Review][Patch] 耳语通知错误触发铃铛红点（决策：排除红点 + 实现绿点）— 铃铛未读计数排除 whisper（仅 tap+knock），并在侧边栏图标为有未读耳语显示绿点（UX-DR20 / AC-1）[egosync-app/src/App.tsx:55, egosync-app/src/components/layout/Sidebar.tsx:95, egosync-app/src/hooks/useNotifications.ts:73]
- [x] [Review][Patch] 敲门卡片未复用 ActionCard 且按钮偏离 AC-3（决策：重构复用 ActionCard）— 按 Task 11.2 重构为复用 `ActionCard.tsx`，「立即处理/稍后」对齐 confirm/reject 语义 [egosync-app/src/components/butler/ButlerView.tsx:28-95]
- [x] [Review][Patch] 敲门声音提示未实现 — 仅有设置开关，无代码在 knock 到达时读取 `notification.knock_sound` 并播放提示音（AC-3 / Task 12.2，Dev Notes 已给 Web Audio 实现）[egosync-app/src/App.tsx, egosync-app/src/components/butler/ButlerView.tsx]
- [x] [Review][Patch] 声音设置默认值与规格相反 — 默认开启（`useState(true)` + `value !== 'false'`），规格 Task 12.1 / AC-3 要求默认关闭 [egosync-app/src/components/settings/GlobalSettingsModal.tsx:61, egosync-app/src/components/settings/GlobalSettingsModal.tsx:109]
- [x] [Review][Patch] 每日敲门上限用 UTC 日期而非本地日期 — `count_knock_today` 用 `date(created_at)=date('now')`（均 UTC），规格 Dev Notes 明确「本地日期」，UTC+8 用户重置点为本地 08:00 [egosync-app/src-tauri/src/db/notifications.rs:83-91]
- [x] [Review][Patch] 前端/命令测试缺失 — Task 13.4 要求 `useNotifications.test.ts`、Task 13.3 要求 `commands/notification.rs` 测试，均缺失 [egosync-app/src/hooks/useNotifications.ts, egosync-app/src-tauri/src/commands/notification.rs]
- [x] [Review][Defer] 每日敲门上限存在竞态 — count-then-insert 非原子，多个角色并发 spawn 时第 4+ 次 knock 可能漏过限制；低优先级（调度节奏下影响小） [egosync-app/src-tauri/src/services/notification_service.rs:82-93] — deferred, 低优先级并发边界
- [x] [Review][Defer] mark_read 对已读通知返回 NotFound 语义误导 — 应幂等或用 ValidationError；当前前端有 `!isRead` 守卫，无实际影响 [egosync-app/src-tauri/src/db/notifications.rs:58-62] — deferred, 低优先级代码异味
