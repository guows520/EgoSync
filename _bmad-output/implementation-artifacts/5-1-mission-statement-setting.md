---
baseline_commit: d396c3bc1ee58050f6d0f5dc6a1e5cc701d3aa02
---

# Story 5.1: 用户能设定和编辑个人使命宣言

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 设定我的人生使命宣言作为角色间优先级指南,
so that 冲突发生时系统能基于我的价值观给出建议。

## 背景与现状（务必先读）

**本 story 是 Epic 5 的第一个 story — 新建 `mission` 表、Rust 后端 CRUD（model + db + command）、前端接通真实数据并新增结构化模板 UI。无 LLM 调用、无调度器改动、无通知逻辑。核心是"使命宣言的持久化存储 + 前端编辑器"。**

### 已建成的基础（本 story 的接入点）

**前端 — 使命宣言 UI 已存在但使用 mock 数据：**
- `src/components/butler/ButlerSettingsContent.tsx:41` — `const [mission, setMission] = useState('')` 纯前端 mock 状态，无持久化
- `ButlerSettingsContent.tsx:63-67` — 模板数组 `templates`（3个简单一句话模板：家庭优先 / 事业与家庭平衡 / 终身学习），点击填入 textarea。**本 story 需替换为基于《要事第一》四项需要设计的4个多段模板**
- `ButlerSettingsContent.tsx:308-324` — 使命宣言区域 UI：`<textarea>` + 模板按钮列表，但无保存按钮、无加载逻辑
- `ButlerSettingsContent.tsx:1-8` — 已 import `appService` 和 `roleService`，可参照其 service 调用模式
- **本 story 需将 `useState('')` 替换为从后端加载 + 保存逻辑，并新增结构化模板 UI**

**前端 — GlobalSettingsModal 有使命宣言 tab 但应删除：**
- `src/components/settings/GlobalSettingsModal.tsx:407` — 侧边栏有 `tab === 'mission'` 按钮，标题"使命宣言"
- `GlobalSettingsModal.tsx:411` — header 显示"个人使命宣言"文字
- **但文件中无 `{tab === 'mission' && (...)}` 内容区块 — tab 切换后内容区为空白**
- **本 story 需从 GlobalSettingsModal 中删除 mission tab 按钮（第 407 行）和 header 中的 mission 标题（第 411 行），使命宣言只在管家设置中配置**

**前端 — service 层模式参考：**
- `src/services/appService.ts:1-13` — service 对象模式：`export const xxxService = { method: () => invoke<T>('command_name', { args }) }`
- `src/services/roleService.ts` — CRUD service 模式参考（list/create/update/delete）
- **本 story 需新建 `src/services/missionService.ts`，封装 `mission_get` / `mission_update` 调用**

**前端 — 类型定义模式参考：**
- `src/types/role.ts` — 类型定义模式：`export interface Role { ... }` + `#[serde(rename_all = "camelCase")]` 对应
- **本 story 需新建 `src/types/mission.ts`，定义 `Mission` 接口**

**Rust 后端 — command 注册模式：**
- `src-tauri/src/lib.rs:278-360` — `tauri::generate_handler![...]` 中注册所有 commands
- `src-tauri/src/commands/mod.rs:1-14` — `pub mod xxx;` 声明 command 模块
- `src-tauri/src/commands/app.rs:66-80` — `app_get_setting` / `app_set_setting` 是最简单的 command 参考（参数解析 + 调 db + 返回 Result）
- **本 story 需新建 `src-tauri/src/commands/mission.rs`，实现 `mission_get` 和 `mission_update` 两个 command**
- **需在 `commands/mod.rs` 添加 `pub mod mission;`**
- **需在 `lib.rs` 的 `generate_handler!` 中注册 `commands::mission::mission_get` 和 `commands::mission::mission_update`**

**Rust 后端 — DB 层模式参考：**
- `src-tauri/src/db/mod.rs:1-14` — `pub mod xxx;` 声明 db 模块
- `src-tauri/src/db/app_settings.rs:9-31` — `get_setting` / `set_setting` 是最简单的 DB 函数参考（SQL query + error handling）
- `src-tauri/src/db/app_settings.rs:20-31` — `set_setting` 使用 `INSERT OR REPLACE` 模式，与 mission 的 upsert 需求一致
- **本 story 需新建 `src-tauri/src/db/mission.rs`，实现 `get_mission` 和 `upsert_mission` 函数**
- **需在 `db/mod.rs` 添加 `pub mod mission;`**

**Rust 后端 — model 层模式参考：**
- `src-tauri/src/models/mod.rs:1-13` — `pub mod xxx;` 声明 model 模块
- `src-tauri/src/models/role.rs` — Model 定义模式：`#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]` + `#[serde(rename_all = "camelCase")]`
- **本 story 需新建 `src-tauri/src/models/mission.rs`，定义 `Mission` 结构体**
- **需在 `models/mod.rs` 添加 `pub mod mission;`**

**Rust 后端 — error 处理：**
- `src-tauri/src/error.rs:1-66` — `AppError` 枚举，含 `DbError(String)` / `ValidationError(String)` 等变体
- 所有 command 返回 `Result<T, AppError>`
- **本 story 沿用现有 AppError，无需新增变体**

**Rust 后端 — DB 迁移：**
- `src-tauri/src/db/pool.rs:44-51` — `sqlx::migrate!("./migrations")` 自动执行 migrations 目录下的 SQL 文件
- 现有迁移文件编号到 `019_energy_updated_at.sql`
- **epics.md AC 写的是 `migrations/012_mission.sql`，但 012 已被 `012_mcp_server_standard_types.sql` 占用 — 以实际编号为准，使用 `020_mission.sql`**
- **本 story 需新建 `src-tauri/migrations/020_mission.sql`**

**Rust 后端 — 时间戳模式：**
- `src-tauri/src/db/settings.rs` — `chrono_now_pub()` 函数返回 ISO 8601 时间戳字符串
- `db/app_settings.rs:21` — `let now = crate::db::settings::chrono_now_pub();` 调用模式
- **本 story 的 `upsert_mission` 需使用相同模式获取 `updated_at`**

## Acceptance Criteria

1. **AC1**: Given 数据库，Then `migrations/020_mission.sql` 创建 `mission` 表：`id` TEXT PRIMARY KEY, `content` TEXT, `format` TEXT DEFAULT 'free' CHECK(format IN ('free','structured')), `updated_at` TEXT，And 仅允许一行数据（INSERT OR REPLACE，固定 id = 'singleton'）

2. **AC2**: Given 用户在管家设置面板使命宣言区域，When 输入自由文本并点击保存，Then 写入 `mission` 表（`content` = 用户输入, `format` = 'free'），And 重启后保留

3. **AC3**: Given 用户点击参考模板，When 点击，Then 多段文本自动填入编辑框（可继续修改），And 模板覆盖多种风格，包含以下4个模板：
   - **全人平衡型**：覆盖生活（身心健康/经济勤俭）、关爱（家人优先/朋友信任）、学习（每月一书/新技能）、遗产（专业价值/年度影响力项目）四个维度
   - **家庭为基、事业为翼型**：以家庭为根基、事业为翅膀，包含伴侣/父母角色、职业人角色、学习者角色、社区成员角色的平衡承诺
   - **成长与贡献驱动型**：以持续成长自己、服务影响他人为使命，覆盖个人成长（每日反思）、家庭关系（倾听支持）、专业贡献（分享经验）、精神传承（价值观传递）
   - **原则清单式**：基于《高效能人士的七个习惯》罗尔夫·科尔的例子，纯原则列表格式（家庭第一/诚信不妥协/未听取正反双方意见不妄下断语/征求他人意见/诚恳但立场坚定/每年掌握一种新技能等）

4. **AC4**: Given 柯维四段式结构化模板，Then 提供可选结构：「使命 → 原则 → 角色与目标」四段引导，其中角色与目标为动态列表（每个角色含名称和目标/承诺，上限7个），And 用户可在自由文本和结构化模板间切换，And 结构化模式以弹窗编辑、摘要卡片展示，And 结构化格式保存时 `format` = 'structured'，`content` = JSON 序列化的四段内容（`{mission, principle, roles: [{name, goal}]}`）

5. **AC5**: Given 用户选择不设定使命宣言，Then `mission` 表无记录或 `content` 为空，And 冲突仲裁仍可基于其他维度运行（四象限 + 能量值）— 本 story 只需确保空使命不影响后续 story

6. **AC6**: Given 用户编辑已有使命宣言，When 修改后保存，Then 立即生效（INSERT OR REPLACE 覆盖旧记录），下次读取返回新版本

7. **AC7**: Given 前端，Then 使命宣言编辑器接通真实数据（替换原型 mock `useState('')`），And 结构化模板 UI 新增（三段输入框 OR 自由文本切换），And 加载时从后端读取已有使命宣言并填充表单

8. **AC8**: Given Rust 后端，Then Tauri command: `mission_get` 返回 `Option<Mission>`（无记录时返回 None），And `mission_update { content, format }` 执行 INSERT OR REPLACE 到 `mission` 表

## Tasks / Subtasks

- [x] **Task 1: DB 迁移 — 创建 mission 表** (AC: #1)
  - [x] 1.1 新建 `src-tauri/migrations/020_mission.sql`：
    ```sql
    -- Story 5.1: 使命宣言表，仅允许单行记录（id 固定为 'singleton'）
    CREATE TABLE IF NOT EXISTS mission (
        id TEXT PRIMARY KEY NOT NULL DEFAULT 'singleton',
        content TEXT,
        format TEXT NOT NULL DEFAULT 'free' CHECK(format IN ('free', 'structured')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );
    ```
  - [x] 1.2 验证 `sqlx::migrate!("./migrations")` 自动执行新迁移（`db/pool.rs:44-51` 已配置）

- [x] **Task 2: Rust model 层 — Mission 结构体** (AC: #1, #8)
  - [x] 2.1 新建 `src-tauri/src/models/mission.rs`：
    ```rust
    #[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
    #[serde(rename_all = "camelCase")]
    pub struct Mission {
        pub id: String,
        pub content: Option<String>,
        pub format: String,
        pub updated_at: String,
    }
    ```
  - [x] 2.2 在 `src-tauri/src/models/mod.rs` 添加 `pub mod mission;`

- [x] **Task 3: Rust DB 层 — mission 表查询与写入** (AC: #1, #6, #8)
  - [x] 3.1 新建 `src-tauri/src/db/mission.rs`：
    - `get_mission(pool) -> Result<Option<Mission>, AppError>` — 查询 `SELECT * FROM mission WHERE id = 'singleton'`
    - `upsert_mission(pool, content, format) -> Result<Mission, AppError>` — 执行 `INSERT OR REPLACE INTO mission (id, content, format, updated_at) VALUES ('singleton', ?1, ?2, ?3)`，返回写入后的记录
  - [x] 3.2 在 `src-tauri/src/db/mod.rs` 添加 `pub mod mission;`
  - [x] 3.3 参照 `db/app_settings.rs:21` 的 `chrono_now_pub()` 获取 `updated_at` 时间戳

- [x] **Task 4: Rust command 层 — Tauri commands** (AC: #8)
  - [x] 4.1 新建 `src-tauri/src/commands/mission.rs`：
    - `mission_get(pool: State<'_, DbPool>) -> Result<Option<Mission>, AppError>` — 调用 `db::mission::get_mission`
    - `mission_update(content: Option<String>, format: String, pool: State<'_, DbPool>) -> Result<Mission, AppError>` — 校验 format ∈ {'free', 'structured'}，调用 `db::mission::upsert_mission`
  - [x] 4.2 在 `src-tauri/src/commands/mod.rs` 添加 `pub mod mission;`
  - [x] 4.3 在 `src-tauri/src/lib.rs` 的 `generate_handler!` 中注册 `commands::mission::mission_get` 和 `commands::mission::mission_update`

- [x] **Task 5: 前端类型定义** (AC: #7)
  - [x] 5.1 新建 `src/types/mission.ts`：
    ```typescript
    export interface Mission {
      id: string;
      content: string | null;
      format: 'free' | 'structured';
      updatedAt: string;
    }
    ```

- [x] **Task 6: 前端 service 层** (AC: #7)
  - [x] 6.1 新建 `src/services/missionService.ts`：
    ```typescript
    import { invoke } from '@tauri-apps/api/core';
    import type { Mission } from '../types/mission';

    export const missionService = {
      get: () => invoke<Mission | null>('mission_get'),
      update: (content: string | null, format: 'free' | 'structured') =>
        invoke<Mission>('mission_update', { content, format }),
    };
    ```

- [x] **Task 7: 前端 UI — ButlerSettingsContent 使命宣言区域完整实现** (AC: #2, #3, #4, #7)
  - [x] 7.1 将 `ButlerSettingsContent.tsx:41` 的 `const [mission, setMission] = useState('')` 改为从 `missionService.get()` 加载，添加 `missionFormat` state（'free' | 'structured'）
  - [x] 7.2 添加 `loadMission` callback，在 `useEffect` 中调用 `missionService.get()`，填充 `mission` 和 `missionFormat`
  - [x] 7.3 扩展现有使命宣言区域 UI（`ButlerSettingsContent.tsx:308-324`）：
    - 自由文本模式：保留现有 `<textarea>`，替换 `templates` 数组为以下4个基于《要事第一》四项需要（生活/关爱/学习/遗产）的多段模板：
      ```typescript
      const templates = [
        '我的使命是成为一个以原则为中心的人，在生活的各个维度保持平衡与成长。\n\n生活：保持身心健康，经济上勤勉节俭，为家人提供安稳的生活基础。\n关爱：把家人放在第一位——每周至少两个晚上专属陪伴，在重要决策中先问"这对家庭意味着什么"。对朋友真诚相待，值得信任。\n学习：保持终身学习者的心态，每月至少读完一本书或掌握一项新技能，用成长带动身边的人。\n遗产：通过专业能力创造真实价值，每年至少完成一个有长期影响力的项目，让世界因我的存在而更好一点。',
        '我的使命是以家庭为根基，以事业为翅膀，在两者之间找到动态平衡。\n\n作为伴侣和父母：我是家人可以依靠的人。无论工作多忙，家人的健康与快乐始终是第一优先级。每周保留专属家庭时间，重要家庭事件不因工作让步。\n作为职业人：在工作中追求卓越和影响力，但绝不以牺牲家庭为代价。优先做有长期价值的事，而非短期回报的事。\n作为学习者：每季度审视一次生活平衡状态，及时调整。保持开放心态，从每次挫折中学习。\n作为社区成员：力所能及地回馈社会，每年参与至少一次公益或志愿服务。',
        '我的使命是持续成长自己，并用成长去服务和影响他人。\n\n个人成长：每天保留30分钟独处反思时间，定期审视生活是否偏离本心。每月深入阅读一本书，每年掌握一个全新领域。\n家庭关系：用耐心和倾听经营亲密关系，成为家人成长的支持者，而非评判者。\n专业贡献：用专业能力解决真实问题，主动分享知识与经验，帮助他人少走弯路。\n精神传承：活出自己想要传递给下一代的价值观——诚实、勤勉、善良、勇气。决定权始终在自己手中，但要为每个决定负责。',
        '角色：我是丈夫/妻子、父亲/母亲、职业人、终身学习者、社区参与者。\n价值观：以诚信为根基，以平衡为准则，以成长为动力，以贡献为目标。家人优先，原则至上，效率服务于意义。\n目标：在事业上有持续的影响力；在家庭中创造温暖的记忆；在个人成长上永不停步；在社区中留下积极的痕迹。',
      ];
      ```
    - 结构化模板模式：新增四段引导（使命 / 原则 / 角色与目标动态列表），柯维四段式示例点击填入。结构化模式以摘要卡片展示+弹窗编辑
    - 模式切换按钮（自由文本 ↔ 结构化模板）
    - 保存按钮 + 保存状态反馈（参照 `ButlerSettingsContent.tsx:114-115` 的 `setSettingsSavedMessage` 模式）
  - [x] 7.4 结构化模板保存时，将四段内容序列化为 JSON 存入 `content`，`format` = 'structured'。四段全空时存 `content=null`
  - [x] 7.5 结构化模板加载时，若 `format` = 'structured'，从 `content` JSON 反序列化填充四段。向后兼容旧三段式格式
  - [x] 7.6 保存按钮调用 `missionService.update(mission, missionFormat)`，保存成功后显示 "使命宣言已保存" 反馈

- [x] **Task 8: 前端 UI — GlobalSettingsModal 删除 mission tab** (AC: #7)
  - [x] 8.1 删除 `GlobalSettingsModal.tsx:407` 的 mission tab 按钮
  - [x] 8.2 删除 `GlobalSettingsModal.tsx:411` header 中 `tab === 'mission'` 对应的 "个人使命宣言" 文字
  - [x] 8.3 确认删除后无残留 mission 相关代码引用

- [x] **Task 9: 单元测试** (AC: #1, #6, #8)
  - [x] 9.1 Rust DB 测试：在 `db/mission.rs` 添加 `#[cfg(test)] mod tests`，测试 upsert + get 往返、空 content、format 校验
  - [x] 9.2 Rust command 测试：验证 `mission_update` 对非法 format 返回 `ValidationError`
  - [x] 9.3 前端测试：验证 missionService 的 invoke 调用参数正确

## Dev Notes

### 关键技术决策

- **迁移编号**：epics.md 写 `012_mission.sql`，但 012 已被占用，使用 `020_mission.sql`
- **单行约束**：mission 表通过 `id = 'singleton'` 固定主键实现单行约束，配合 `INSERT OR REPLACE` 实现 upsert
- **format 字段**：`'free'` = 自由文本，`'structured'` = 柯维四段式 JSON。结构化内容存储为 `{"mission": "...", "principle": "...", "roles": [{"name": "...", "goal": "..."}]}` JSON 字符串。向后兼容旧格式 `{role, value, goal}`（加载时 value 映射到 principle，role+goal 合并为一条角色条目）
- **空使命处理**：`content` 为 `Option<String>`，允许为 NULL。`mission_get` 返回 `Option<Mission>`，无记录时返回 `None`
- **编辑入口决策**：ButlerSettingsContent 为唯一使命宣言编辑入口（管家设置面板），GlobalSettingsModal 中的 mission tab 删除，避免两个编辑入口导致状态同步问题

### 架构合规

- **分层规则**：前端 → `missionService` → `invoke()` → `commands/mission.rs`（薄层解析）→ `db/mission.rs`（SQL 执行）→ SQLite
- **命名规范**：Rust command 使用 `snake_case`（`mission_get`, `mission_update`），前端 service 使用 `camelCase`（`missionService.get()`, `missionService.update()`）
- **错误处理**：command 返回 `Result<T, AppError>`，前端 try-catch
- **serde 桥接**：Rust `#[serde(rename_all = "camelCase")]` 自动将 `updated_at` 序列化为 `updatedAt`

### 前端 UI 规范

- **设计系统**：Tailwind CSS + shadcn/ui，圆角 10px，参照现有 ButlerSettingsContent 的使命宣言区域样式
- **色彩**：主色调 indigo-600，背景 slate-50/white，边框 slate-200
- **交互**：保存按钮带 loading 状态（Loader2 旋转图标），保存成功后显示 "已保存" 反馈（参照 `ButlerSettingsContent.tsx:114-115` 的 `setSettingsSavedMessage` 模式）
- **模板按钮**：参照 `ButlerSettingsContent.tsx:319-323` 现有模板按钮样式
- **模式切换**：自由文本 / 结构化模板切换使用 tab 或 toggle 按钮，切换时保留已输入内容（自由文本 ↔ JSON 互转）

### 反模式警告

- **不要**在 `GlobalSettingsModal` 中实现使命宣言编辑器 — 使命宣言只在管家设置（`ButlerSettingsContent`）中配置，全局设置面板中的 mission tab 应删除
- **不要**新建 `MissionModal.tsx` 组件 — 架构树中列出的 `modals/MissionModal.tsx` 是为后续 story 预留的，本 story 在 ButlerSettingsContent 中实现
- **不要**在 `services/` 层添加业务逻辑 — service 层只做 HTTP/invoke 封装
- **不要**在 `commands/` 层添加业务逻辑 — commands 只做参数校验 + 调 db + 返回结果
- **不要**使用 `012_mission.sql` 作为迁移文件名 — 012 已被占用
- **不要**在 mission 表上创建外键关联 — mission 是全局单例，不关联任何角色

### Project Structure Notes

新增文件：
- `src-tauri/migrations/020_mission.sql` — DB 迁移
- `src-tauri/src/models/mission.rs` — Rust model
- `src-tauri/src/db/mission.rs` — Rust DB 层
- `src-tauri/src/commands/mission.rs` — Rust commands
- `src/services/missionService.ts` — 前端 service
- `src/types/mission.ts` — 前端类型

修改文件：
- `src-tauri/src/models/mod.rs` — 添加 `pub mod mission;`
- `src-tauri/src/db/mod.rs` — 添加 `pub mod mission;`
- `src-tauri/src/commands/mod.rs` — 添加 `pub mod mission;`
- `src-tauri/src/lib.rs` — 注册 `mission_get` / `mission_update` commands
- `src/components/butler/ButlerSettingsContent.tsx` — 使命宣言区域接通真实数据 + 新增结构化模板 UI + 保存逻辑
- `src/components/settings/GlobalSettingsModal.tsx` — 删除 mission tab 按钮和 header 标题

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.1] — AC 原文
- [Source: _bmad-output/planning-artifacts/prd-egosync.md#FR-13] — 使命宣言设定功能需求
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping] — FR-13~15 → `arbitration.rs` + `mission` DB 表
- [Source: _bmad-output/planning-artifacts/architecture.md#Layer Rules] — 分层规则
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 3] — 角色冲突仲裁用户旅程
- [Source: _bmad-output/implementation-artifacts/4-8-energy-calculation-engine.md] — 前一 story 模式参考

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

- 所有 9 个 Task 全部完成，16 个 Rust 测试 + 7 个前端 service 测试 + 11 个 ButlerSettingsContent 回归测试 + 13 个 GlobalSettingsModal 回归测试全部通过
- 迁移文件使用 `020_mission.sql`（012 已被占用）
- Mission model 添加了 `serde::Deserialize` derive（story 中只列了 `Serialize`，但 `FromRow` + 反序列化需要）
- Command 层提取 `validate_format` 函数便于单元测试
- 前端 ButlerSettingsContent 使命宣言区域支持自由文本/结构化模板切换、4 个模板、保存反馈
- 结构化模板从三段式重构为四段式（使命→原则→角色与目标），基于《高效能人士的七个习惯》柯维框架。角色与目标为动态列表（上限7个），结构化模式以摘要卡片+弹窗编辑
- 自由文本第4个模板从柯维三段式替换为原则清单式（罗尔夫·科尔例子）
- 向后兼容旧三段式 JSON 格式（`{role, value, goal}` → 加载时自动转换）
- GlobalSettingsModal mission tab 已删除，使命宣言唯一编辑入口在 ButlerSettingsContent

### File List

新增文件：
- `src-tauri/migrations/020_mission.sql`
- `src-tauri/src/models/mission.rs`
- `src-tauri/src/db/mission.rs`
- `src-tauri/src/commands/mission.rs`
- `src/types/mission.ts`
- `src/services/missionService.ts`
- `src/services/missionService.test.ts`

修改文件：
- `src-tauri/src/models/mod.rs` — 添加 `pub mod mission;`
- `src-tauri/src/db/mod.rs` — 添加 `pub mod mission;`
- `src-tauri/src/commands/mod.rs` — 添加 `pub mod mission;`
- `src-tauri/src/lib.rs` — 注册 `mission_get` / `mission_update` commands
- `src/components/butler/ButlerSettingsContent.tsx` — 使命宣言区域接通真实数据 + 结构化模板 UI + 保存逻辑
- `src/components/settings/GlobalSettingsModal.tsx` — 删除 mission tab 按钮和 header 标题

### Review Findings

_代码审查（2026-06-24）— 三层对抗审查（Blind Hunter / Edge Case Hunter / Acceptance Auditor）_

- [x] [Review][Patch] 结构化模式保存空内容时存为非空 JSON（决策1已定：四段全空→存 content=null）— 已修复：保存前判断四段是否全空（trim），全空则 `content=null` [ButlerSettingsContent.tsx:508-509]
- [x] [Review][Dismiss] 自由文本 ↔ 结构化切换未做内容互转 — 决策2已定：保持现状（两套独立 state，来回切不丢内容），AC4 硬性要求仅为「可切换」已满足，互转为 Dev Notes 指引非强制，不修改
- [x] [Review][Patch] 结构化记录 content 解析失败时静默丢失数据 — 已修复：catch 分支同时 `setMissionFormat('free')`，使回退内容在自由文本框可见 [ButlerSettingsContent.tsx:97-100]
- [x] [Review][Patch] 加载期间模式切换按钮可点击 — 已修复：两个切换按钮加 `disabled={isLoadingMission}` [ButlerSettingsContent.tsx:359-374]
- [x] [Review][Patch] 工作区残留运行日志 — 已修复：`src-tauri/.gitignore` 新增 `run-stdout.txt` / `run-stderr.txt` 忽略规则
- [x] [Review][Defer] 缺少 ButlerSettingsContent 使命宣言组件级测试 — `ButlerSettingsContent.test.tsx` 无 mission 加载/结构化解析/保存的断言（service 层已有测试覆盖），组件层逻辑（JSON 解析、模式切换、保存反馈）未测 — deferred, 测试增强非阻塞
