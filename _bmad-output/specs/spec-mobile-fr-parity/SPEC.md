---
id: SPEC-mobile-fr-parity
companions:
  - fr-groups.md
  - ../../implementation-artifacts/spec-companion-android-icon-parity.md
sources:
  - ../../implementation-artifacts/investigations/mobile-desktop-parity-investigation.md
  - ../../implementation-artifacts/deferred-work.md
---

> **Canonical contract.** This SPEC and the files in `companions:` are the complete, preservation-validated contract for what to build, test, and validate. Source documents listed in frontmatter are for traceability only — consult them only if you need narrative rationale or prose color this contract intentionally omits.

# 移动端 FR 屏补全（15 项）

## Why

移动端（companion-android）24 项一/二档 FR 中 12 项完全缺失、3 项仅部分实现，用户无法在手机上体验桌面端已完整交付的核心交互（对话路由、任务拆分、角色切换、记忆管理、仪表盘筛选等）。图标/主题/导航壳/数据接线已就绪（commit f638a7c 图标 + 设计语言 + SnapshotStore 重接线），缺失纯粹是"屏/交互没做"。本 spec 将 15 项按 7 屏分组蒸馏为可独立交付的 story 粒度，使移动端与桌面在功能/布局/图标/体验上「同一个产品」。

## Capabilities

- id: CAP-1
  intent: 对话屏（chat）——用户可通过移动端对话完成意图路由、任务分配、角色切换、推理溯源、不确定性表达、工具执行可视，与桌面 ChatStream 功能等价。
  success: chat 组 6 项（FR-1/2/20/29/30/33）的桌面基线逐项对照通过（功能/布局/图标），`./gradlew :app:assembleDebug` BUILD SUCCESSFUL。

- id: CAP-2
  intent: 仪表盘屏（dashboard）——用户可在角色卡仪表盘设置主动性刻度盘，并按时段/角色筛选统计数据。
  success: dashboard 组 2 项（FR-12/38）的桌面基线逐项对照通过，构建成功。

- id: CAP-3
  intent: 角色/记忆屏（role/memory）——用户可在引导阶段确认角色涌现，并在记忆屏查看来源溯源、执行选择性遗忘。
  success: role/memory 组 3 项（FR-5/8/9）的桌面基线逐项对照通过，构建成功。

- id: CAP-4
  intent: 任务屏（tasks）——用户可查看任务分类中状态与象限筛选交互，补全自动四象限分类的部分实现。
  success: tasks 组 1 项（FR-23）的桌面基线逐项对照通过，构建成功。

- id: CAP-5
  intent: 周复盘屏（review）——用户可在周复盘中执行大石头规划（排期/勾选/移除），补全只读展示为完整规划交互。
  success: review 组 1 项（FR-17）的桌面基线逐项对照通过，构建成功。

- id: CAP-6
  intent: 引导屏（onboarding）——用户在未配对/无角色时获得空状态引导，完成初始设置流。（2026-09-10 裁决失效：手机端引导流已移除，spec-companion-android-remove-onboarding）
  success: onboarding 组 1 项（FR-21）的桌面基线逐项对照通过，构建成功。（2026-09-10 裁决失效：组 6 已随手机端引导流移除退役，spec-companion-android-remove-onboarding）

- id: CAP-7
  intent: 通知屏（notify）——用户在任务屏与通知中心看到 Q2 保护提醒（徽章+通知条目）。
  success: notify 组 1 项（FR-24）的桌面基线逐项对照通过，构建成功。

## Constraints

- 依赖仅限 Compose BOM / Material3 / Navigation-Compose / kotlinx-coroutines + 既有 material-icons-core；禁引入 Room / Hilt / OkHttp / 任何网络库 / material-icons-extended。
- 数据继续跑 mock / SnapshotStore 只读，不碰后端（桌面 companion Rust 模块 Phase 2/3 整体后延）。
- 图标一律使用已落地的 `ui/icons/LucideIcons.kt`（40 枚）+ `RoleIcons.kt`（24-id 白名单 + getRoleIcon 兜底），禁止新增 emoji。
- 18 项 tier-1/2 FR「严格逐项一致」（功能/布局/图标/交互可逐项映射桌面，仅做移动适配）；FR-20/21「强制语义适配」（移动无侧栏/全屏引导尺寸限制）。（2026-09-10 裁决失效：FR-21 移动侧引导已移除，spec-companion-android-remove-onboarding）
- FR-14/15/7 不在本 spec 范围内（已冻结裁决：语义一致，经后端事件→通知承载，不在本前端工作内）。
- 每组交付后 `./gradlew :app:assembleDebug` BUILD SUCCESSFUL + `RoleIconsTest` 回归通过。
- 每屏分组为一个独立 story，不做单次全量交付。

## Non-goals

- 不实现桌面 companion Rust 模块（pairing/connection/snapshot/dispatch）——Phase 2/3 后延。
- 不接入真实连接层或 COMMAND 指令通道。
- 不修改已完成的设计语言（密度/动效/色温）、图标系统、SnapshotStore 重接线。
- 不新增后端驱动功能（FR-14 冲突检测、FR-15 三步仲裁、FR-7 记忆自动提炼）。
- 不做视觉像素级比对（无渲染环境，仅代码级/构建级验证）。

## Success signal

15 项 FR 全部在移动端有对应屏/交互实现，`./gradlew :app:assembleDebug` BUILD SUCCESSFUL，`RoleIconsTest` 通过，24-FR 覆盖矩阵重跑后「完整」数 ≥ 21（原 6 + 新增 12 缺失 + 3 部分升级）。用户人审确认各屏布局/图标/体验与桌面「同一个产品」。（2026-09-10 裁决失效：手机端引导流已移除——「15 项 FR 全部实现」口径改为 14 项（FR-21 移动侧退役）、「完整」数 ≥ 21 下调为 ≥ 20（括号内算式为历史口径）；本行其余断言（构建/RoleIconsTest/人审确认）不受影响——spec-companion-android-remove-onboarding）

## Assumptions

- FR-33（Agent Loop 工具执行可视）归入 chat 组，其移动形态为聊天屏加工具执行过程可视区（对齐桌面 ChatStream.tsx:529），不含 @Skill 工作目录（桌面优先三档，不进移动）。
- 「严格逐项一致」标准高于规格默认（architecture.md:1939 仅规定分档+通道收敛），以用户裁决为准。
- 桌面基线以调查 B 表所列 path:line 为准；若桌面代码后续有变更，以实施时实际代码为准。

</parameter>
</function>