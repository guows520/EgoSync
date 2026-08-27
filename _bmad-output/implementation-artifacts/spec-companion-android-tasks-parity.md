---
title: 'companion-android 任务屏补齐：归属筛选 + 新建任务 + 只看大石头'
type: 'feature'
created: '2026-05-27'
status: 'done'
baseline_commit: 'bad7634'
---

# companion-android 任务屏补齐：归属筛选 + 新建任务 + 只看大石头

## Intent

**Problem:** 移动端任务 Tab 相对桌面母本（TaskOverviewTab.tsx）缺三项能力：按管家/角色归属筛选任务、新建任务、"只看大石头"开关。根因是数据模型层 TaskItem 只有 roleName 展示字符串，无结构化归属（桌面为 ownerType + roleId），筛选无从做起；新建则完全无入口。

**Approach:** 镜像桌面语义补齐三层：Store 层 TaskItem 增加 ownerType/roleId 结构化归属字段（mock 同步）；VM 层补 ownerFilter（多选排除语义，对齐桌面 deselectedOwners）、showBigRocksOnly、createTask（"智能判断"象限复用既有 classifyingIds 过渡态机制）；UI 层加归属 chip 筛选行、大石头开关、"新增任务"入口 + 底部弹层表单（字段镜像桌面 TaskModal：归属/标题/象限含智能判断/截止/大石头）。

## Boundaries & Constraints

**Always:** 桌面端为唯一事实源——筛选语义（多选排除）、表单字段集、classifying 过渡态行为逐项镜像 TaskOverviewTab.tsx 与 TaskModal.tsx；Tailwind 风格的 M3 组件承载；新建任务在 mock 层本地追加（真实层 COMMAND 帧注释保留）；遵循既有命名与文件组织规范。

**Ask First:** 若实现中发现 classifyingIds 契约（TasksUiStateTest 锁定）需要破坏性变更才能支持新建流程。

**Never:** 不做删除/编辑任务（用户未声称，另案处理）；不引入持久化框架或真实网络层；不做已完成折叠（桌面有但用户未要求）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 归属筛选 | 取消勾选"管家"+ 勾选角色 A | 仅显示角色 A 的任务（管家任务被排除） | N/A |
| 归属筛选全不选 | 所有归属都被取消 | 列表显示空态（与桌面 deselectedOwners 全排除时一致） | 空态文案 |
| 新建-智能判断 | 标题填入、象限选"智能判断"、提交 | 任务立即以"分类中"徽章出现，4 秒后归类到某象限（mock 模拟桌面 task:classified 事件） | N/A |
| 新建-空标题 | 提交时标题为空 | 表单不关闭，标题输入框报错提示（镜像桌面 handleSubmit 校验） | 行内报错 |
| 新建-指定象限 | 象限选 Q1-Q4 之一 | 任务直接落入对应象限分组，无分类中状态 | N/A |
| 只看大石头 | 开关开启 | 仅显示 bigRock=true 的任务（与象限筛选、归属筛选叠加） | N/A |
| 归属选项源 | roles 列表 | 管家 + 全部角色（对齐桌面 ownerOptions 顺序） | N/A |

## Code Map

- `companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt` -- TaskItem 数据模型（:39-50）、mock 任务（:366-373 管家字符串约定）、roles（选项源）
- `companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt` -- TasksUiState（quadrantFilter :21-26）、classifying 机制（:50-69，有测试契约锁定）
- `companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt` -- 象限筛选行（:127-168）、TaskRow、离线提示
- `companion-android/app/src/test/java/com/egosync/companion/ui/tasks/TasksUiStateTest.kt` -- 既有契约测试（classifying :61-105）
- 桌面母本：`egosync-app/src/components/butler/TaskOverviewTab.tsx:184-208`（归属筛选）、`:276-282`（新建入口）、`egosync-app/src/components/modals/TaskModal.tsx:101-148`（表单）、`egosync-app/src/types/task.ts:8-13`（ownerType/roleId）

## Tasks & Acceptance

**Execution:**
- [x] `SnapshotStore.kt` -- TaskItem 增加 ownerType: TaskOwner（BUTLER/ROLE）+ roleId: String?，mock 数据同步（管家任务改结构化表达）-- 筛选的稳定标识基础
- [x] `SnapshotStore.kt` -- 增加 mock 新增任务支持（可变列表或 VM 层持有 + store 补充接口）-- 新建落地
- [x] `TasksViewModel.kt` -- TasksUiState 增加 deselectedOwners: Set<String>、showBigRocksOnly: Boolean、grouped() 扩展双过滤、toggleOwner/toggleBigRocksOnly -- 筛选状态机
- [x] `TasksViewModel.kt` -- createTask(title, ownerType, roleId, quadrant?, due?, bigRock) + "智能判断"复用 classifyingIds -- 新建逻辑
- [x] `TasksScreen.kt` -- 归属筛选 chip 行（Users 图标 + 管家/各角色多选）+ 只看大石头开关 -- 筛选 UI
- [x] `TasksScreen.kt` -- "新增任务"入口 + 底部弹层表单（归属/标题/象限含智能判断/截止/大石头）-- 新建 UI
- [x] `TasksUiStateTest.kt` -- 新增单测：归属过滤、大石头过滤叠加、新建智能判断走 classifying、新建指定象限直接归组 -- 锁定 WHY 契约

**Acceptance Criteria:**
- Given 任务列表含管家+多角色任务，when 取消管家勾选，then 仅角色任务可见
- Given 新建表单，when 标题空提交，then 表单不关且报错
- Given 新建选"智能判断"，when 提交，then 出现分类中徽章且 4s 后归组（镜像桌面异步分类）
- Given 只看大石头开启 + Q2 筛选 + 管家排除，then 三条件叠加过滤生效

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:testDebugUnitTest` -- expected: BUILD SUCCESSFUL（含新增单测全绿）

## Suggested Review Order

**数据模型（归属结构化）**

- 桌面 task.ts 的 ownerType/roleId 语义逐字镜像，管家键常量贯穿三层
  [`SnapshotStore.kt:40`](../../companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt#L40)

**筛选状态机（三重过滤）**

- 分类中任务豁免象限过滤：占位象限 ≠ 最终归类，保证「立即上屏」契约在任何筛选下成立
  [`TasksViewModel.kt:36`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt#L36)

- toggleOwner solo-from-default 语义逐字镜像桌面（默认态点单个 = 只看它）
  [`TasksViewModel.kt:53`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt#L53)

**新建编排（智能分类过渡态）**

- createTask：智能判断走 classifyingIds + 4s mock 事件；显式象限直接归组
  [`TasksViewModel.kt:147`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt#L147)

- id 序列从种子推导，消除硬编码手工同步
  [`TasksViewModel.kt:203`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt#L203)

- 任务不在列表时分类事件为 no-op，不用过期快照复活
  [`TasksViewModel.kt:171`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt#L171)

**表单 UI（弹层 + 离线门禁 + 生命周期）**

- 保存按钮离线禁用：弹层在独立 window 绕过降级蒙层，需与入口同步门禁
  [`TasksScreen.kt:470`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt#L470)

- 弹层可见性与草稿 rememberSaveable：旋转/重建不丢
  [`TasksScreen.kt:101`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt#L101)

- 标题校验抽纯函数（空/纯空格 → 与桌面逐字一致的报错文案）
  [`TasksScreen.kt:216`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt#L216)

- 新建入口 Role.Button 语义（TalkBack 可达）
  [`TasksScreen.kt:114`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt#L114)

**测试（锁定 WHY 契约）**

- 分类中任务不受象限筛选隐藏（「立即上屏」契约的筛选边界）
  [`TasksUiStateTest.kt:221`](../../companion-android/app/src/test/java/com/egosync/companion/ui/tasks/TasksUiStateTest.kt#L221)

- 空标题校验纯函数（提交时校验分支唯一守门人）
  [`TasksUiStateTest.kt:233`](../../companion-android/app/src/test/java/com/egosync/companion/ui/tasks/TasksUiStateTest.kt#L233)
