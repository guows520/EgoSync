---
baseline_commit: 81885e9
---

# Story 11.1: 查看并筛选跨 Agent 活动统计

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 管家用户,
I want 在仪表盘查看任务、记忆、对话和待处理任务统计，并按 Agent 与时间范围筛选,
So that 我能快速了解整体或特定 Agent 在指定时期内的活动情况。

**FRs covered:** FR-38

## Acceptance Criteria

> 来源：`_bmad-output/planning-artifacts/epics.md` 第 2892-2979 行。AC 编号为本 Story 内部编号，用于 Tasks 引用。

**AC-1 四项指标展示与默认值**
**Given** 用户打开管家仪表盘
**When** 活动统计加载成功
**Then** 展示任务总数、结构化记忆条目数、对话会话数和待处理任务数
**And** 默认统计全部 Agent、全部时间
**And** 现有角色状态卡片和排序行为保持不变。

**AC-2 会话计数语义**
**Given** 同一会话包含多条消息
**When** 系统计数对话
**Then** 该会话只计为 1，不按消息数累计。

**AC-3 任务计数语义**
**Given** 查询范围内存在已完成和未完成任务
**When** 系统计数
**Then** 任务总数包含范围内全部任务
**And** 待处理任务只包含查询时仍未完成的任务
**And** `pendingTaskCount <= taskCount`。

**AC-4 Agent 筛选器**
**Given** 用户打开 Agent 筛选器
**When** 查看或选择筛选项
**Then** 可选择"全部""管家"或任一当前角色
**And** 默认选择"全部"
**And** 四项指标只统计所选作用域且不重复计数。

**AC-5 归档角色安全回退**
**Given** 当前选中角色被归档或删除
**When** Dashboard 重新加载
**Then** 筛选安全回退到"全部"
**And** 不继续提交失效角色 ID。

**AC-6 默认时间范围**
**Given** 用户未指定时间范围
**When** 查询执行
**Then** 默认统计全部时间，不设置隐式历史截断日期。

**AC-7 有效时间范围筛选**
**Given** 用户选择有效开始和结束时间
**When** 查询执行
**Then** 任务和记忆按 `created_at` 纳入范围
**And** 会话按 `started_at` 纳入范围
**And** 待处理任务先按 `created_at` 纳入，再按查询时未完成状态计数。

**AC-8 组合筛选一致性**
**Given** 用户同时选择 Agent 和时间范围
**When** 查询执行
**Then** 两个条件同时作用于四项指标
**And** 四项指标基于同一筛选快照更新。

**AC-9 时间边界半开区间**
**Given** 查询使用 RFC 3339 时间值
**When** 记录位于时间边界
**Then** 所有数据源统一使用半开区间 `[from, to)`
**And** 相邻范围不重复或遗漏边界记录。

**AC-10 无效输入校验**
**Given** 用户提交不存在的角色、开始晚于结束、无法解析的时间字符串
**When** 后端验证
**Then** 请求被拒绝并返回明确错误
**And** 不执行部分聚合
**And** 前端保留上一次成功结果。
**Note** 单边时间范围（仅 `startAt` 或仅 `endAt`）视为有效：仅 `startAt` 表示 `[startAt, +∞)`，仅 `endAt` 表示 `(-∞, endAt)`；“不完整”指字符串无法解析为 RFC 3339，而非缺少一个边界。

**AC-11 空数据显示**
**Given** 任一指标没有匹配数据
**When** 结果返回
**Then** 对应指标显示为 0，不隐藏指标。

**AC-12 双数据库全有或全无失败**
**Given** 主数据库或会话数据库查询失败
**When** 聚合服务生成结果
**Then** 整个统计请求明确失败
**And** 不混合展示部分真实、部分默认值的数据。

**AC-13 快速切换竞态保护**
**Given** 用户快速切换筛选条件
**When** 多个请求先后返回
**Then** 只展示最新条件对应的结果
**And** 旧请求不得覆盖新结果
**And** 加载期间保留上一次成功统计。

**AC-14 IPC 边界与数据访问**
**Given** 前端请求活动统计
**When** 调用 Tauri IPC
**Then** 使用统一查询 DTO、聚合结果 DTO 和 Tauri Command
**And** React 不直接访问数据源
**And** 会话通过现有 Repository/Adapter 边界参与聚合。

**AC-15 自动化测试覆盖**
**Given** Story 11.1 自动化测试运行
**When** 执行 Repository、Service、Command、Hook、组件及关键 E2E 测试
**Then** 覆盖四项指标、会话计数、组合筛选、时间边界、无效条件、空数据与双数据库失败行为。

## Tasks / Subtasks

> 任务按架构路径自后端 DTO/DB 层到 Service/Command、再到前端、最后测试的顺序排列。每条标注涉及的 AC。所有"修改"为外科手术式扩展，不重构相邻代码。

### 阶段 1：后端 DTO 与 Model 层

- [ ] **Task 1: 新增 DashboardMetrics Query/Scope/Metrics DTO** (AC: #1, #4, #6, #7, #9, #14)
  - [ ] 1.1 在 `src-tauri/src/models/dashboard.rs` 新增以下结构体（保留现有 `DashboardStatus` 不修改）：
    ```rust
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(tag = "type", rename_all = "camelCase")]
    pub enum DashboardMetricsScope {
        All,
        Butler,
        Role { role_id: String },
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DashboardMetricsQuery {
        pub scope: DashboardMetricsScope,
        pub start_at: Option<String>,
        pub end_at: Option<String>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DashboardMetrics {
        pub task_count: i64,
        pub memory_count: i64,
        pub conversation_count: i64,
        pub pending_task_count: i64,
        pub generated_at: String,
    }
    ```
    - **tagged union**：`scope` 使用 `#[serde(tag = "type")]` 实现显式 tagged union（`architecture.md` 第 1404 行），前端序列化为 `{ type: "all" }`、`{ type: "butler" }`、`{ type: "role", roleId: "..." }`
  - [ ] 1.2 验证：`cargo check` 编译通过

### 阶段 2：DB 层新增指标聚合查询

- [ ] **Task 2: tasks.rs 新增指标计数函数** (AC: #1, #3, #7, #9)
  - [ ] 2.1 在 `src-tauri/src/db/tasks.rs` 新增：
    ```rust
    /// 仪表盘指标聚合：按 owner 条件和时间范围返回 (task_count, pending_task_count)
    pub async fn count_tasks_for_metrics(
        pool: &SqlitePool,
        owner_condition: &str,    // "all" | "butler" | "role"
        role_id: Option<&str>,
        start_at: Option<&str>,
        end_at: Option<&str>,
    ) -> Result<(i64, i64), AppError>
    ```
    - SQL：`SELECT COUNT(*) as task_count, SUM(CASE WHEN is_completed = 0 THEN 1 ELSE 0 END) as pending_task_count FROM tasks WHERE deleted_at IS NULL AND {owner_filter} AND {time_filter}`
    - `owner_filter`：`all` → `1=1`；`butler` → `owner_type = 'butler' AND role_id IS NULL`；`role` → `owner_type = 'role' AND role_id = ?`
    - `time_filter`：均为 `None` → `1=1`；仅 `start_at` → `created_at >= ?`；仅 `end_at` → `created_at < ?`；两者 → `created_at >= ? AND created_at < ?`（半开区间 `[start_at, end_at)`，AC-9）
    - 返回 `(task_count, pending_task_count)`，保证 `pending_task_count <= task_count`（AC-3）
  - [ ] 2.2 单元测试覆盖：全量计数、管家作用域、角色作用域、时间范围内、时间范围外、半开区间边界、空数据返回 (0, 0)

- [ ] **Task 3: memories.rs 新增记忆计数函数** (AC: #1, #7, #9)
  - [ ] 3.1 在 `src-tauri/src/db/memories.rs` 新增：
    ```rust
    /// 仪表盘指标聚合：按 owner 条件和时间范围返回 memory_count
    pub async fn count_memories_for_metrics(
        pool: &SqlitePool,
        owner_condition: &str,
        role_id: Option<&str>,
        start_at: Option<&str>,
        end_at: Option<&str>,
    ) -> Result<i64, AppError>
    ```
    - SQL：`SELECT COUNT(*) FROM memories WHERE {owner_filter} AND {time_filter}`
    - `owner_filter`：`all` → `1=1`；`butler` → `role_id IS NULL`；`role` → `role_id = ?`
    - `time_filter`：同 Task 2，使用 `created_at` 字段，半开区间
  - [ ] 3.2 单元测试覆盖：全量计数、管家作用域、角色作用域、时间范围、空数据返回 0

- [ ] **Task 4: conversations.rs 新增会话计数函数** (AC: #1, #2, #7, #9)
  - [ ] 4.1 在 `src-tauri/src/db/conversations.rs` 新增：
    ```rust
    /// 仪表盘指标聚合：按 owner 条件和时间范围返回 conversation_count（按会话计数，不按消息）
    pub async fn count_conversations_for_metrics(
        pool: &ConversationsPool,
        owner_condition: &str,
        role_id: Option<&str>,
        start_at: Option<&str>,
        end_at: Option<&str>,
    ) -> Result<i64, AppError>
    ```
    - SQL：`SELECT COUNT(*) FROM conversations WHERE {owner_filter} AND {time_filter}`
    - `owner_filter`：`all` → `1=1`；`butler` → `role_id IS NULL`；`role` → `role_id = ?`
    - `time_filter`：使用 `started_at` 字段（AC-7），半开区间 `[start_at, end_at)`（AC-9）
    - **按会话计数，不 JOIN messages 表**（AC-2）
  - [ ] 4.2 单元测试覆盖：全量计数、管家作用域、角色作用域、时间范围、多消息会话只计 1（AC-2）、空数据返回 0

### 阶段 3：Service 层 — 跨库聚合

- [ ] **Task 5: DashboardAggregationService 新增聚合函数** (AC: #1, #3, #8, #10, #11, #12, #14)
  - [ ] 5.1 在 `src-tauri/src/services/dashboard_service.rs` 新增：
    ```rust
    pub async fn get_dashboard_metrics(
        pool: &SqlitePool,
        conv_pool: &ConversationsPool,
        query: DashboardMetricsQuery,
    ) -> Result<DashboardMetrics, AppError>
    ```
    - **输入验证**（AC-10）：
      - `scope = Role { role_id }` 时验证角色存在（`roles::get_role_by_id`），不存在返回 `ValidationError`
      - `start_at` 和 `end_at` 均存在时验证 `start_at < end_at`，否则返回 `ValidationError`
      - `start_at`/`end_at` 非空时验证可解析为 RFC 3339 时间，否则返回 `ValidationError`
    - **作用域映射**（`architecture.md` 第 1429-1433 行）：
      - `All`：不加 Agent 条件，统计全部非删除业务记录
      - `Butler`：任务使用 `owner_type = 'butler' AND role_id IS NULL`；记忆和会话使用 `role_id IS NULL`
      - `Role { role_id }`：任务使用 `owner_type = 'role' AND role_id = ?`；记忆和会话使用 `role_id = ?`
    - **跨库并行聚合**（`architecture.md` 第 1437 行）：
      - 主库通过 `tokio::try_join!` 并行执行 `count_tasks_for_metrics` 和 `count_memories_for_metrics`
      - 会话库执行 `count_conversations_for_metrics`
      - 三者使用同一规范化查询快照（scope + 时间范围）
    - **全有或全无失败**（AC-12）：任一数据源失败则整个请求返回错误，不使用零值掩盖
    - **空数据**（AC-11）：任一指标无匹配数据时对应值为 0
    - `generated_at` 使用当前 UTC 时间 RFC 3339 格式
  - [ ] 5.2 **不修改** 现有 `get_dashboard_status` 函数（回归边界，AC-1）
  - [ ] 5.3 单元测试覆盖：
    - 四项指标全量计数
    - 管家作用域
    - 角色作用域
    - 组合筛选（Agent + 时间范围，AC-8）
    - 时间边界半开区间
    - 无效角色 ID 被拒绝
    - 开始晚于结束被拒绝
    - 空数据返回全 0
    - 主库失败时整个请求失败（AC-12）
    - 会话库失败时整个请求失败（AC-12）

### 阶段 4：Tauri Command 层

- [ ] **Task 6: 新增 dashboard_get_metrics Command** (AC: #14)
  - [ ] 6.1 在 `src-tauri/src/commands/dashboard.rs` 新增：
    ```rust
    #[tauri::command]
    pub async fn dashboard_get_metrics(
        query: DashboardMetricsQuery,
        pool: State<'_, DbPool>,
        conv_pool: State<'_, ConversationsPool>,
    ) -> Result<DashboardMetrics, AppError> {
        dashboard_service::get_dashboard_metrics(&pool, &conv_pool, query).await
    }
    ```
    - Command 只做参数反序列化和委托调用，不含业务逻辑（`architecture.md` 第 1736 行）
  - [ ] 6.2 在 `src-tauri/src/lib.rs` 的 `generate_handler!` 宏中注册 `dashboard_get_metrics`（在现有 `dashboard_get_status` 之后）
  - [ ] 6.3 在 `src-tauri/src/models/mod.rs` 中导出新 DTO（如需要）
  - [ ] 6.4 验证：`cargo check` 编译通过

### 阶段 5：前端类型与 Service

- [ ] **Task 7: 扩展前端 Dashboard 类型** (AC: #14)
  - [ ] 7.1 在 `src/types/dashboard.ts` 新增（保留现有 `DashboardStatus` 不修改）：
    ```typescript
    export type DashboardMetricsScope =
      | { type: 'all' }
      | { type: 'butler' }
      | { type: 'role'; roleId: string };

    export interface DashboardMetricsQuery {
      scope: DashboardMetricsScope;
      startAt: string | null;
      endAt: string | null;
    }

    export interface DashboardMetrics {
      taskCount: number;
      memoryCount: number;
      conversationCount: number;
      pendingTaskCount: number;
      generatedAt: string;
    }
    ```

- [ ] **Task 8: 扩展 dashboardService** (AC: #14)
  - [ ] 8.1 在 `src/services/dashboardService.ts` 新增：
    ```typescript
    getMetrics: (query: DashboardMetricsQuery) =>
      invoke<DashboardMetrics>('dashboard_get_metrics', { query }),
    ```
    - 保留现有 `getStatus` 方法不变

### 阶段 6：前端 Hook — 筛选与竞态保护

- [ ] **Task 9: 扩展 useDashboard Hook** (AC: #1, #4, #6, #10, #11, #12, #13)
  - [ ] 9.1 修改 `src/hooks/useDashboard.ts`，新增指标查询状态管理：
    - 新增 state：`metrics`（`DashboardMetrics | null`）、`metricsLoading`、`metricsError`
    - 新增 `scope` state（`DashboardMetricsScope`，默认 `{ type: 'all' }`，AC-4）
    - 新增 `timeRange` state（`{ startAt: string | null; endAt: string | null }`，默认全 `null`，AC-6）
    - 新增 `fetchMetrics` 函数：调用 `dashboardService.getMetrics({ scope, ...timeRange })`
    - **竞态保护**（AC-13）：使用请求 ID 或 `AbortController`，只接受最新请求的结果，旧请求返回时丢弃
    - **失败保留旧数据**（AC-13）：刷新失败时保留上一次成功的 `metrics`，设置 `metricsError`
    - **首次失败显示错误**（`architecture.md` 第 1443 行）：`metrics` 为 null 时显示错误
    - `useEffect` 依赖 `scope` 和 `timeRange`，变化时自动刷新
  - [ ] 9.2 **不修改** 现有 `statuses`/`isLoading`/`error` 逻辑（回归边界，AC-1）
  - [ ] 9.3 新增 `setScope` 和 `setTimeRange` 暴露给组件

### 阶段 7：前端 UI — 指标卡与筛选器

- [ ] **Task 10: DashboardTab 新增指标卡和筛选 UI** (AC: #1, #4, #5, #6, #10, #11, #12, #13)
  - [ ] 10.1 在 `src/components/butler/DashboardTab.tsx` 新增：
    - **指标卡区域**：四张卡片展示 `taskCount`、`memoryCount`、`conversationCount`、`pendingTaskCount`（AC-1）
      - 使用 Lucide 图标（如 `ListTodo`、`Brain`、`MessageSquare`、`Clock`）
      - 空数据显示为 0（AC-11）
      - 加载中显示 skeleton/spinner
      - 错误时显示错误提示（首次）或保留旧数据 + 更新失败标记（AC-13）
    - **Agent 筛选器**：下拉选择"全部"/"管家"/各活跃角色（AC-4）
      - 默认选择"全部"（AC-4）
      - 选项来自现有 `statuses`（活跃角色列表）
      - **归档角色回退**（AC-5）：当 `scope` 指向的角色不在当前活跃列表中时，自动回退到 `{ type: 'all' }`
    - **时间范围选择器**：开始和结束日期/时间输入
      - 默认为空（全部时间，AC-6）
      - 前端负责将本地日历选择换算为 RFC 3339 绝对时间点（`architecture.md` 第 1404 行）
    - **保留现有角色状态卡片区域和排序行为**（AC-1）：指标卡和筛选器在角色状态卡之上，不修改下方区域
  - [ ] 10.2 使用 Tailwind utility class 样式，禁止自定义 CSS（`project-context.md` 第 92 行）
  - [ ] 10.3 **不新增组件到 App.tsx**（`project-context.md` 第 147-148 行）

### 阶段 8：测试

- [ ] **Task 11: Rust 后端测试** (AC: #1, #2, #3, #7, #9, #10, #11, #12, #15)
  - [ ] 11.1 `db/tasks.rs` 测试：`count_tasks_for_metrics` 覆盖全量/管家/角色作用域、时间范围、半开区间边界、空数据 (0, 0)、`pending <= total` 不变量
  - [ ] 11.2 `db/memories.rs` 测试：`count_memories_for_metrics` 覆盖全量/管家/角色作用域、时间范围、空数据返回 0
  - [ ] 11.3 `db/conversations.rs` 测试：`count_conversations_for_metrics` 覆盖全量/管家/角色作用域、时间范围、多消息会话只计 1、空数据返回 0
  - [ ] 11.4 `services/dashboard_service.rs` 测试：`get_dashboard_metrics` 覆盖四项指标全量、组合筛选、时间边界、无效角色 ID 拒绝、开始晚于结束拒绝、空数据全 0、主库失败整个请求失败、会话库失败整个请求失败
  - [ ] 11.5 `commands/dashboard.rs` 测试：`dashboard_get_metrics` Command 参数传递和错误传播
  - [ ] 11.6 测试 DB 初始化（`setup_test_db`）需包含 `memories` 表和 `conversations` 表 schema

- [ ] **Task 12: 前端组件测试** (AC: #1, #4, #5, #10, #11, #12, #13, #15)
  - [ ] 12.1 `useDashboard.test.ts` 新增测试：
    - 默认加载指标（scope=all, timeRange=null）
    - 切换 scope 触发刷新
    - 切换 timeRange 触发刷新
    - 竞态保护：快速切换 scope 只接受最新结果（AC-13）
    - 刷新失败保留旧数据（AC-13）
    - 首次失败显示错误
  - [ ] 12.2 `DashboardTab.test.tsx` 新增测试：
    - 四项指标卡渲染（含空数据显示 0，AC-11）
    - Agent 筛选器选项和默认值（AC-4）
    - 切换筛选触发指标刷新
    - 归档角色回退到"全部"（AC-5）
    - 加载状态显示
    - 错误状态显示（首次 vs 刷新失败保留旧数据，AC-13）
  - [ ] 12.3 `DashboardTab.a11y.test.tsx` 新增测试：
    - 指标卡可访问性（aria-label）
    - 筛选器键盘操作和读屏

- [ ] **Task 13: 回归验证** (AC: #1)
  - [ ] 13.1 现有 `useDashboard.test.ts` 全部通过（角色状态卡逻辑不变）
  - [ ] 13.2 现有 `DashboardTab.test.tsx` 全部通过（角色状态卡渲染不变）
  - [ ] 13.3 现有 `dashboard_service.rs` 测试全部通过（`get_dashboard_status` 不变）

**测试命令**：
```powershell
cd egosync-app/src-tauri && cargo check
cd egosync-app/src-tauri && cargo test
cd egosync-app && npx vitest run
cd egosync-app && npx tsc --noEmit
cd egosync-app && npm run build
```

**测试纪律**：任一测试被跳过或环境缺失时，必须明确记录，不能宣称全部通过（AGENTS 规则十二：显式失败）。

### Review Findings

- [x] [Review][Patch] 允许单边时间范围，并修订 AC-10 对“不完整”的定义：仅 `startAt` 表示从该时间起，仅 `endAt` 表示该时间前；用户已确认此产品语义 [_bmad-output/implementation-artifacts/11-1-view-and-filter-cross-agent-activity-statistics.md:79]
- [x] [Review][Patch] RFC 3339 偏移时间未经 UTC 规范化便参与 SQLite 文本比较，等价时间点可能被错误纳入或排除，违反 AC-7/AC-9 [egosync-app/src-tauri/src/services/dashboard_service.rs:90]
- [x] [Review][Patch] 角色失效回退在 React 渲染阶段调用 `setScope`，且指标请求不以活跃角色快照为发送前置条件，无法可靠保证 AC-5 的“不继续提交失效角色 ID” [egosync-app/src/components/butler/DashboardTab.tsx:44]
- [x] [Review][Patch] 指标刷新期间 UI 用省略号替换已有成功值，未满足 AC-13“加载期间保留上一次成功统计” [egosync-app/src/components/butler/DashboardTab.tsx:139]
- [x] [Review][Patch] `datetime-local` 的值使用 UTC 字符串回填，本地非 UTC 时区选择后会发生可见时间偏移 [egosync-app/src/components/butler/DashboardTab.tsx:110]
- [x] [Review][Patch] 指标 effect 卸载时未使请求失效，异步完成后仍可能更新已卸载组件状态 [egosync-app/src/hooks/useDashboard.ts:49]
- [x] [Review][Patch] 缺少 AC-12 主数据库失败与会话数据库失败的全有或全无测试 [egosync-app/src-tauri/src/services/dashboard_service.rs:378]
- [x] [Review][Patch] AC-13 快速切换竞态、刷新失败保留旧值及 AC-5 归档角色回退测试缺失 [egosync-app/src/hooks/useDashboard.test.ts:89]
- [x] [Review][Patch] AC-15 指定的 Command 参数/错误传播测试和关键 E2E 用户旅程尚未实现 [egosync-app/src-tauri/src/commands/dashboard.rs:15]
- [x] [Review][Patch] 时间解析与指标错误分支存在重复逻辑，应合并为单次解析和单一错误赋值路径 [egosync-app/src-tauri/src/services/dashboard_service.rs:90]
- [x] [Review][Patch] 修复 `DashboardMetricsScope::Role` 变体字段 `role_id` 未被 camelCase 重命名导致前端 `roleId` 反序列化失败的真实 IPC bug [egosync-app/src-tauri/src/models/dashboard.rs:19]

## Dev Notes

### 核心架构决策（`architecture.md` 第 1388-1449 行）

**决策：保留现有角色状态接口，新增独立 `dashboard_get_metrics` 聚合接口。**

现有 `dashboard_get_status` 继续返回角色状态卡，不修改 `DashboardStatus[]` 契约。新增 `dashboard_get_metrics` 独立返回四项聚合指标。

### API 与 DTO（`architecture.md` 第 1392-1416 行）

查询作用域使用显式 tagged union：`all`、`butler`、`role { roleId }`；时间范围为可空的 `{ startAt, endAt }`。`null` 表示全部时间，指定范围统一使用 RFC 3339 半开区间 `[startAt, endAt)`，且必须满足 `startAt < endAt`。前端负责根据用户本地日历边界换算绝对时间点。

返回 DTO：

```text
DashboardMetrics {
  taskCount: i64,
  memoryCount: i64,
  conversationCount: i64,
  pendingTaskCount: i64,
  generatedAt: RFC3339
}
```

### 指标语义（`architecture.md` 第 1418-1427 行）

| 指标 | 数据源 | 时间字段 | 条件 |
|---|---|---|---|
| `taskCount` | 主库 `tasks` | `created_at` | `deleted_at IS NULL` |
| `memoryCount` | 主库 `memories` | `created_at` | 无 |
| `conversationCount` | `ConversationsPool` | `started_at` | 按会话计数，不按消息计数 |
| `pendingTaskCount` | 主库 `tasks` | `created_at` | `deleted_at IS NULL AND is_completed = 0` |

待处理任务定义为"在筛选时间内创建、查询执行时仍未完成"，因此必须满足 `pendingTaskCount <= taskCount`，不使用 `completed_at` 回溯历史时点状态。

### 作用域映射（`architecture.md` 第 1429-1433 行）

- `all`：不加 Agent 条件，统计全部非删除业务记录；不依赖活跃角色列表，避免角色停用后历史数据从总数消失。
- `butler`：任务使用 `owner_type = 'butler' AND role_id IS NULL`；记忆和会话使用 `role_id IS NULL`。
- `role`：任务使用 `owner_type = 'role' AND role_id = :role_id`；记忆和会话使用相同 `role_id`。后端验证角色存在，前端默认只展示活跃角色作为筛选项。

### 聚合与一致性（`architecture.md` 第 1435-1439 行）

新增 `DashboardAggregationService`：主库通过单个 SQL statement 返回任务总数、记忆数和待处理任务数；会话计数通过 Conversations Repository/Adapter 查询。两部分可用 `tokio::try_join!` 并行执行，复用同一规范化查询快照。

由于主库与 ConversationsPool 是独立 SQLite 数据库，本轮承诺同一请求的近实时一致性，不承诺跨数据库事务级原子快照；不 attach 数据库、不复制会话数据、不建立统计缓存或物化视图。

### 失败策略（`architecture.md` 第 1441-1445 行）

新指标接口采用全有或全无：任一数据源失败则整个请求返回错误，不使用零值掩盖失败。前端首次失败显示错误；已有成功数据刷新失败时保留上一次结果并显示更新失败状态。

成功日志仅记录规范化筛选条件、生成时间与耗时；失败日志额外记录失败数据源，不记录任务、记忆或会话正文。本轮不新增查询审计表。

### 性能策略（`architecture.md` 第 1447-1449 行）

不预先添加猜测性索引。实现后使用实际数据和 `EXPLAIN QUERY PLAN` 验证；仅在出现证据充分的扫描瓶颈时，增加与作用域和时间字段匹配的最小组合索引。

### 数据流（`architecture.md` 第 1727-1736 行）

```text
DashboardTab → useDashboard → dashboardService.getMetrics
→ dashboard_get_metrics → DashboardAggregationService
├── tasks/memories：主 DbPool
└── conversations：ConversationsPool
```

UI 管理筛选和展示；Hook 管理查询、刷新、loading/error 和旧结果保留；Command 仅反序列化参数；`dashboard_service.rs` 是 scope 和时间规范化的唯一位置；DB 模块只执行已规范化的统计 SQL。现有 `dashboard_get_status` 继续只负责角色状态卡。

### 数据边界（`architecture.md` 第 1749-1753 行）

主 `DbPool` 继续保存 Skill、绑定、MCP Server、管家/角色 MCP 绑定、任务和记忆。`conversations` 与 `messages` 继续位于独立 `ConversationsPool`。

`DashboardAggregationService` 可以同时读取两个连接池，但不得跨库写入、复制会话数据或让前端分别调用多个计数接口自行聚合；不承诺跨数据库事务级快照。

### 现有代码当前状态（必须阅读后再修改）

**后端 Dashboard 链路**：

| 文件 | 当前行 | 当前状态 | 本 Story 修改 |
|---|---|---|---|
| `models/dashboard.rs` 第 1-13 行 | `DashboardStatus` 结构体 | 角色状态卡 DTO | 新增 `DashboardMetricsScope`、`DashboardMetricsQuery`、`DashboardMetrics` |
| `services/dashboard_service.rs` 第 10-71 行 | `get_dashboard_status` | 角色状态卡查询 | 新增 `get_dashboard_metrics`；**不修改**现有函数 |
| `commands/dashboard.rs` 第 8-14 行 | `dashboard_get_status` | 角色状态卡 Command | 新增 `dashboard_get_metrics` |
| `lib.rs` 第 345-369 行 | `generate_handler!` | 已注册 `dashboard_get_status` | 新增 `dashboard_get_metrics` 注册 |

**DB 层现有查询**：

| 文件 | 当前行 | 当前状态 | 本 Story 修改 |
|---|---|---|---|
| `db/tasks.rs` 第 596-644 行 | `get_task_stats_for_roles` | 按角色批量查 pending/urgent | 新增 `count_tasks_for_metrics`（按 scope + 时间范围） |
| `db/memories.rs` 第 389-401 行 | `count_memories` | 按 role_id + category 计数 | 新增 `count_memories_for_metrics`（按 scope + 时间范围） |
| `db/conversations.rs` 第 423-448 行 | `get_last_active_for_roles` | 按角色查最近活跃 | 新增 `count_conversations_for_metrics`（按 scope + 时间范围） |

**前端 Dashboard 链路**：

| 文件 | 当前行 | 当前状态 | 本 Story 修改 |
|---|---|---|---|
| `types/dashboard.ts` 第 1-11 行 | `DashboardStatus` | 角色状态卡 TS 类型 | 新增 `DashboardMetricsScope`、`DashboardMetricsQuery`、`DashboardMetrics` |
| `services/dashboardService.ts` 第 4-6 行 | `getStatus` | 角色状态卡 Service | 新增 `getMetrics(query)` |
| `hooks/useDashboard.ts` 第 7-40 行 | `useDashboard` | 角色状态卡 Hook | 新增 metrics/scope/timeRange 状态和 `fetchMetrics`；**不修改**现有逻辑 |
| `components/butler/DashboardTab.tsx` 第 30-99 行 | 角色状态卡渲染 | 角色状态卡 UI | 新增指标卡和筛选器区域；**不修改**角色状态卡区域 |

### 现有可复用能力（禁止重复实现）

1. **角色状态卡链路**：`get_dashboard_status` → `dashboard_get_status` → `useDashboard.getStatus` → `DashboardTab` 角色卡区域，全部复用，不修改
2. **tasks 表结构**：`owner_type`/`role_id`/`is_completed`/`deleted_at`/`created_at` 字段已存在，直接使用
3. **memories 表结构**：`role_id`/`created_at` 字段已存在，直接使用
4. **conversations 表结构**：`role_id`/`started_at` 字段已存在，直接使用
5. **ConversationsPool**：`db/conversations.rs` 已封装所有会话库访问，新增函数在同一文件
6. **AppError**：现有错误类型（`ValidationError`、`DbError`、`NotFound`）直接复用

### 必须保留的回归边界

**后端**：
- `get_dashboard_status` 函数行为不变
- `dashboard_get_status` Command 行为不变
- `DashboardStatus` 结构体不修改
- `get_task_stats_for_roles` 不修改
- `count_memories` 不修改
- `get_last_active_for_roles` 不修改

**前端**：
- `DashboardStatus` TS 类型不修改
- `dashboardService.getStatus` 方法签名不变
- `useDashboard` 现有 `statuses`/`isLoading`/`error`/`clearError` 逻辑不变
- `DashboardTab` 现有角色状态卡渲染区域不变

**业务逻辑**：
- 角色状态卡排序逻辑（紧急优先、低能量优先）不变
- 现有角色状态卡数据加载流程不变

### 禁止模式（`architecture.md` 第 1786-1794 行）

- ❌ UI 直接聚合多个数据库计数结果
- ❌ Command 层写 SQL 或含业务逻辑
- ❌ Dashboard Service 读取任务、记忆或会话正文
- ❌ 跨库写入、复制会话数据或建立统计缓存
- ❌ 为指标聚合新增平行的通用框架或目录层级
- ❌ 前端分别调用多个计数接口自行聚合

### Project Structure Notes

- 所有修改遵循 `_bmad-output/project-context.md` 的目录组织、命名规范和分层架构
- 后端遵循三层架构：Command 只解析参数 → Service 含业务逻辑 → DB 只执行 SQL
- 新增 Tauri Command 必须在 Rust Command、`lib.rs generate_handler!`、前端 Service、TypeScript DTO 和测试中闭环（`project-context.md` 第 209-216 行检查清单）
- 新增 DTO 遵循 `#[serde(rename_all = "camelCase")]` 规范，前端 TS 使用 camelCase
- tagged union 使用 `#[serde(tag = "type")]` 实现显式标签

### References

- `_bmad-output/planning-artifacts/epics.md` 第 2886-2979 行 — Epic 11 和 Story 11.1 完整 AC
- `_bmad-output/planning-artifacts/architecture.md` 第 1197-1204 行 — FR-38 需求概述
- `_bmad-output/planning-artifacts/architecture.md` 第 1388-1449 行 — FR-38 核心架构决策
- `_bmad-output/planning-artifacts/architecture.md` 第 1727-1736 行 — FR-38 数据流
- `_bmad-output/planning-artifacts/architecture.md` 第 1749-1753 行 — 数据边界
- `_bmad-output/planning-artifacts/architecture.md` 第 1786-1794 行 — 禁止跨越的边界
- `_bmad-output/planning-artifacts/architecture.md` 第 1656-1660 行 — 文件标记（DashboardTab、useDashboard、dashboardService、dashboard.ts、dashboard.rs、dashboard_service.rs、tasks.rs、memories.rs、conversations.rs）
- `_bmad-output/planning-artifacts/architecture.md` 第 1780-1781 行 — 需求到文件映射
- `_bmad-output/project-context.md` — 技术栈、命名规范、分层架构、禁止事项
- [Source: _bmad-output/planning-artifacts/architecture.md#FR-38：仪表盘统计聚合接口]
- [Source: _bmad-output/planning-artifacts/architecture.md#FR-38：仪表盘统计]

## Dev Agent Record

### Agent Model Used

TBD

### Debug Log References

TBD

### Completion Notes List

TBD

### Change Log

TBD

### File List

TBD
