package com.egosync.companion.ui.chat

import com.egosync.companion.sync.ChatMessage
import com.egosync.companion.sync.DecompositionItem
import com.egosync.companion.sync.ExecutionTraceBlock
import com.egosync.companion.sync.RoleProposal
import com.egosync.companion.sync.TaskDecompositionProposal
import com.egosync.companion.sync.ToolStatus

/**
 * 管家对话演示数据（Story 13.2 从 SnapshotStore mock 迁出）：
 * 回复轮换/委派路由/拆分提案/执行溯源/涌现提案属**生成性内容**（快照口径无，
 * 指令通道 13.3 接入）——不进快照通道，作为 ui 包私有演示数据保留既有交互。
 */

/** 用户发言后管家的流式回复轮换（mock 打字机效果）。 */
val butlerReplies: List<String> = listOf(
    "好的，这事我记下了。要不要让产品经理跟进？完成后第一时间告诉你。",
    "明白。这件事我建议放进 Q2——重要，但不必现在动手。您看呢？",
    "已经为您记下来了。顺带一提，今天 Q1 还有两件事，需要我复述一下吗？",
    "收到，boss。学习者角色这几天能量偏低（33%），要不要安排 20 分钟的读书时间？",
)

/** 委派第一段气泡文案模板（{role} 替换为角色名）。 */
fun delegationFirstSegment(roleName: String): String =
    "稍等，我让${roleName}看一下。"

/** 委派第二段气泡文案模板（角色视角反馈）。 */
val delegationReplies: Map<String, String> = mapOf(
    "role-pm" to "来自产品经理的反馈：这事我接下了。初步看有两个切入点，我先出个方案草稿，明早给你过目。",
    "role-father" to "来自父亲的反馈：收到。我把这件事和女儿的时间表对了一下，不冲突，我来跟进。",
    "role-learner" to "来自学习者的反馈：好的，我把这章的笔记先翻出来，整理个摘要给你参考。",
)

/** 任务拆分提案种子（FR-2，mock 触发：管家视图第 2 轮回复后浮现）。 */
val decompositionProposal: TaskDecompositionProposal = TaskDecompositionProposal(
    id = "td-1",
    taskSummary = "整理季度 OKR 草稿",
    roleName = "产品经理",
    roleIcon = "target",
    items = listOf(
        DecompositionItem("回顾上季度 OKR 完成度", "周四"),
        DecompositionItem("草拟下季度 3 个 O", "周五"),
        DecompositionItem("拆解 KR 并对齐负责人", "下周一"),
    ),
)

/** 执行溯源块种子（FR-29，随第二轮回复挂在助手消息上）。 */
val executionTrace: List<ExecutionTraceBlock> = listOf(
    ExecutionTraceBlock.Thinking(
        id = "trace-think-1",
        content = "用户想整理季度 OKR。先判断范围：上季度复盘 + 下季度目标 + KR 对齐。这是一件可以拆分的复合任务。",
        elapsedSeconds = 4,
        isActive = false,
    ),
    ExecutionTraceBlock.Narration(
        id = "trace-narr-1",
        content = "我先查一下你的日程和现有任务，看看怎么排最合适。",
    ),
    ExecutionTraceBlock.Action(
        id = "trace-action-1",
        title = "读取日程与任务快照",
        status = ToolStatus.COMPLETED,
        actionType = "read",
    ),
)

/** 低置信回复（FR-30：confidence<0.7 → 气泡尾部内联标注）。 */
val lowConfidenceReply: String =
    "我猜你可能是想问下周的安排，但我不太确定具体指哪一天。你可以再说明一下吗？"

/** 工具执行阶段序列（FR-33：流式期间的工具名+状态指示轮换）。 */
val toolExecutionStages: List<String> = listOf(
    "读取日程与任务快照",
    "整理竞品对比要点",
    "生成回复草稿",
)

/** 角色涌现提案种子（FR-5，mock 触发：管家视图第 2 轮回复后浮现提案卡）。 */
val roleProposal: RoleProposal = RoleProposal(
    name = "策划师",
    icon = "lightbulb",
    color = "#8B5CF6",
    goal = "把零散的想法收敛成可执行的方案",
)

// ── Preview 专用种子（不参与快照通道）───────────────────────────────

/** 管家开场消息（Preview 样例，内容对齐原 mock initialChat）。 */
val previewInitialChat: List<ChatMessage> = listOf(
    ChatMessage(
        id = "m-1",
        fromButler = true,
        text = "早上好 boss。今天最重要的一件事是下午 2 点的产品评审，材料产品经理已经备好了。另外，女儿钢琴课在四点半，我提前提醒您。",
    ),
)

/** 角色视图种子消息（Preview 样例）。 */
val previewRoleChatSeeds: Map<String, List<ChatMessage>> = mapOf(
    "role-pm" to listOf(
        ChatMessage(
            id = "r-pm-1",
            fromButler = true,
            senderRoleId = "role-pm",
            text = "boss，评审材料已备好。竞品对比页我标了 3 处需要你拍板的点，会前 30 分钟我会提醒你过一遍。",
        ),
    ),
    "role-father" to listOf(
        ChatMessage(
            id = "r-fa-1",
            fromButler = true,
            senderRoleId = "role-father",
            text = "今天 16:30 是女儿钢琴课。课后建议问问她上课感受，另外这周末画展的票我已经留意了。",
        ),
    ),
    "role-learner" to listOf(
        ChatMessage(
            id = "r-le-1",
            fromButler = true,
            senderRoleId = "role-learner",
            text = "昨晚沉淀了 2 条《深度工作》读书笔记。第 3 章还剩一半，要不要安排 20 分钟读完？",
        ),
    ),
)
