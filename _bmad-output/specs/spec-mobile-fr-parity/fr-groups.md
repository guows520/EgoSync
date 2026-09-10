# FR 屏分组明细（桌面基线 → 移动新增）

本文件为 SPEC.md 的 companion，承载逐 FR 的桌面基线与移动落地要求。每组为一个独立 story，按顺序交付。

---

## 组 1｜chat（CAP-1）：FR-1 / FR-2 / FR-20 / FR-29 / FR-30 / FR-33

### FR-1 意图解析路由（缺失→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `chatService.ts:5` routingMetadata；`ChatStream.tsx:551` 委派两段气泡 |
| 移动新增 | ChatViewModel 接指令通道（mock），渲染「委派中→角色回复」两段气泡 |
| 图标 | 与桌面一致：角色头像 + 委派指示（使用 LucideIcons 中已有角色图标） |

### FR-2 对话分配任务（缺失→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `TaskDecompositionCard.tsx:12-85`（ListTree）对话流内嵌拆分提案卡 |
| 移动新增 | ChatScreen 内嵌 TaskDecompositionCard + 接受/不要拆分按钮 |
| 图标 | ListTree 等效 → 使用 LucideIcons 中 ListTodo 或新增（需评估） |

### FR-20 对话区角色切换（缺失→完整，强制语义适配）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `Sidebar.tsx:6-144` 左 64px 图标栏（Home/Plus/Moon…） |
| 移动新增 | 顶部或抽屉式角色切换器（移动无侧栏，**语义适配**） |
| 图标 | 角色图标使用 RoleIcons.getRoleIcon(id) |
| 适配说明 | 桌面 64px 侧栏→移动改为顶部水平滚动角色选择器或底部抽屉 |

### FR-29 推理溯源（缺失→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `ChatBubble.tsx:236` ExecutionTrace 可折叠（ChevronRight） |
| 移动新增 | ChatBubble 加可折叠执行过程区（Think/Narration/Action） |
| 图标 | ChevronRight → LucideIcons 中 ChevronRight |

### FR-30 不确定性表达（缺失→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `ButlerSettingsContent.tsx:1457` confidence<0.7「仅供参考」 |
| 移动新增 | 对应内联文本（桌面徽章已移除，对齐即可） |
| 图标 | 无额外图标需求 |

### FR-33 Agent Loop 工具执行可视（部分→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `ChatStream.tsx:529` 流式+thinking+工具执行+停止（Play/Square） |
| 移动新增 | ChatScreen 加工具执行过程可视（工具名+状态指示） |
| 图标 | Play/Square 等效 → LucideIcons 中对应 |
| 排除 | @Skill 工作目录属桌面优先三档，不进移动 |

---

## 组 2｜dashboard（CAP-2）：FR-12 / FR-38

### FR-12 主动性刻度盘（缺失→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `ProactivityToggle.tsx:16-37` 三档分段 |
| 移动新增 | 角色/设置页加三档分段控件（SegmentedButton 或等效） |
| 图标 | 无额外图标需求（纯控件） |

### FR-38 仪表盘统计筛选（缺失→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `DashboardTab.tsx:85-228` DateRangeFilter + scope 下拉 + 4 指标卡（CalendarDays） |
| 移动新增 | DashboardScreen 加筛选行（时间/角色筛选）+ 指标网格 |
| 图标 | CalendarDays → LucideIcons 中对应 |

---

## 组 3｜role/memory（CAP-3）：FR-5 / FR-8 / FR-9

### FR-5 角色涌现确认（缺失→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `RoleConfirmModal.tsx:47-269` 图标 24 选 + 色板 8 选（X） |
| 移动新增 | 角色涌现确认屏（引导阶段：图标网格 24 选 + 色板 8 选 + 确认/取消） |
| 图标 | 使用 RoleIcons.kt 的 24 id 白名单 + ROLE_COLORS 8 色 |

### FR-8 记忆查询溯源（缺失→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `MemoryTab.tsx:129-158` toggleSource + 来源消息列表（Clock） |
| 移动新增 | 角色详情/记忆屏加来源展开区（展开/折叠 + 来源消息列表） |
| 图标 | Clock → LucideIcons 中 Clock |

### FR-9 选择性遗忘（缺失→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `MemoryTab.tsx:160-215` 遗忘确认流（确认/再想想） |
| 移动新增 | 记忆屏加遗忘确认流（删除按钮→确认对话框→确认/取消） |
| 图标 | Trash2 → LucideIcons 中对应 |

---

## 组 4｜tasks（CAP-4）：FR-23

### FR-23 自动四象限分类（部分→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `useTasks.ts:29` task:classified + 「智能分类中…」徽章（Filter/Loader2） |
| 移动新增 | TasksScreen 加分类中态（加载指示）+ 象限筛选交互 |
| 图标 | Filter/Loader2 → LucideIcons 中对应 |

---

## 组 5｜review（CAP-5）：FR-17

### FR-17 大石头规划（部分→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `WeeklyReviewModal.tsx:181-262` plan 阶段（Check/Plus/X） |
| 移动新增 | WeeklyReviewScreen 加规划阶段交互（当前只读→可勾选/添加/移除） |
| 图标 | Check/Plus/X → LucideIcons 中已有 |

---

## 组 6｜onboarding（CAP-6）：FR-21（2026-09-10 裁决失效：手机端引导流已移除，spec-companion-android-remove-onboarding）

### FR-21 空状态引导（缺失→完整，强制语义适配）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `OnboardingView.tsx:24-386` + `RoleConfirmModal.tsx:47` |
| 移动新增 | OnboardingScreen（未配对/无角色时引导）——移动全屏引导，**语义适配** |
| 图标 | 使用 LucideIcons 中已有管家/角色图标 |
| 适配说明 | 桌面全屏引导→移动全屏引导（尺寸不同但形态相同） |

---

## 组 7｜notify（CAP-7）：FR-24

### FR-24 Q2 保护提醒（缺失→完整）

| 维度 | 内容 |
|------|------|
| 桌面基线 | `App.tsx:87` q2:reminder 事件；`TaskOverviewTab.tsx:115` amber 徽章+左边框（AlertTriangle） |
| 移动新增 | TasksScreen 任务卡加 Q2 徽章 + 通知中心承载提醒条目 |
| 图标 | AlertTriangle → LucideIcons 中对应 |

---

## 交付顺序建议

1. 组 1 chat（6 项，体量最大）
2. 组 2 dashboard（2 项）
3. 组 3 role/memory（3 项）
4. 组 4 tasks（1 项）
5. 组 5 review（1 项）
6. 组 6 onboarding（1 项）（2026-09-10 裁决失效：手机端引导流已移除，勿再交付——spec-companion-android-remove-onboarding）
7. 组 7 notify（1 项）

## 每组验收标准

- `cd companion-android && ./gradlew :app:assembleDebug` → BUILD SUCCESSFUL
- 该组 FR 的桌面基线逐项对照（功能/布局/图标）
- `RoleIconsTest` 回归通过
- 视觉对比待人审（无渲染环境）

</parameter>
</function>