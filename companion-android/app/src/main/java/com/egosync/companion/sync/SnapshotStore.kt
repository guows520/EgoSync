package com.egosync.companion.sync

import com.egosync.companion.ui.theme.Quadrant
import com.egosync.companion.ui.theme.RoleDomain
import java.time.Instant
import java.time.ZoneOffset
import java.time.format.DateTimeParseException
import java.util.Locale

/**
 * 快照数据模型 + 内存 mock。
 *
 * 对应架构「快照引擎」：手机端对快照只读渲染（桌面为唯一事实源）。
 * 接入真实连接层后，本 object 中的静态 mock 将替换为
 * SNAPSHOT / STATE_DELTA 帧驱动的版本化快照存储。
 */

// ── 角色卡 ─────────────────────────────────────────────────────────────

data class RoleCard(
    val id: String,
    val name: String,
    val icon: String,
    val domain: RoleDomain,
    /** 能量 0~100：任务完成率/大石头推进/互动/目标活跃的加权健康度 */
    val energy: Int,
    val pendingCount: Int,
    val lastActive: String,
    val taskCount: Int,
    val memoryCount: Int,
    val sessionCount: Int,
)

// ── 四象限任务 ─────────────────────────────────────────────────────────

/** Q2 保护状态（FR-24，镜像桌面 types/task.ts:2 TaskProtectionStatus）。 */
enum class TaskProtectionStatus { NORMAL, AT_RISK }

data class TaskItem(
    val id: String,
    val title: String,
    val quadrant: Quadrant,
    val roleName: String,
    val due: String?,
    /** 大石头：本周不可妥协的优先项（每角色每周 1~2 件） */
    val bigRock: Boolean,
    val done: Boolean = false,
    /** Q2 保护提醒（FR-24）：at_risk = 重要任务被持续挤压（母本 q2_protection_reminder.rs 检测）。 */
    val protectionStatus: TaskProtectionStatus = TaskProtectionStatus.NORMAL,
)

// ── 晨间简报 ───────────────────────────────────────────────────────────

data class BriefingSection(val title: String, val body: String)

data class BriefingActionPoint(
    val id: String,
    val text: String,
    val fromRole: String,
    val confirmed: Boolean? = null, // null=待处理 true=已确认 false=已拒绝
)

data class MorningBriefing(
    val date: String,
    val greeting: String,
    val sections: List<BriefingSection>,
    val actionPoints: List<BriefingActionPoint>,
)

// ── 周复盘 ─────────────────────────────────────────────────────────────

data class RoleScore(
    val roleName: String,
    val completed: Int,
    val total: Int,
    val energyNow: Int,
    val energyDelta: Int,
)

data class BigRockResult(
    val text: String,
    val completed: Boolean,
)

data class WeeklyReview(
    val weekLabel: String,
    val narrative: String,
    val roleScores: List<RoleScore>,
    /** 近 7 天能量趋势（聚合值 0~100），供 Canvas 柱状图自绘 */
    val energyTrend: List<Int>,
    val energyTrendDays: List<String>,
    val bigRockResults: List<BigRockResult>,
    val closingQuestion: String,
)

/** 大石头规划条目（FR-17，镜像桌面 types/review.ts BigRockPlanItem）。 */
data class BigRockPlanItem(val roleId: String, val title: String)

// ── 三级通知（whisper / tap / knock）───────────────────────────────────

enum class NoticeLevel(val label: String, val description: String) {
    WHISPER("耳语", "角色日常沉淀，静默积累进简报"),
    TAP("轻触", "值得顺嘴一提的动态"),
    KNOCK("敲门", "需要你决定的事，每日上限 3 次"),
}

data class NoticeItem(
    val id: String,
    val level: NoticeLevel,
    val text: String,
    val fromRole: String,
    val time: String,
    val read: Boolean = false,
    /** 敲门级通知附带待决策 ActionCard */
    val actionable: Boolean = false,
    val actionState: Boolean? = null, // null=待处理 true=已确认 false=已拒绝
)

// ── 管家对话 ───────────────────────────────────────────────────────────

data class ChatMessage(
    val id: String,
    val fromButler: Boolean,
    val text: String,
    /** 流式打字机进行中（管家回复逐字浮现） */
    val streaming: Boolean = false,
    /** 发言人角色 id（FR-1 委派第二段）；null=管家或用户消息 */
    val senderRoleId: String? = null,
    /** 置信度<0.7（FR-30）：视图层气泡尾部内联标注 */
    val lowConfidence: Boolean = false,
)

data class ActionCardSuggestion(
    val id: String,
    val title: String,
    val detail: String,
    val fromRole: String,
    val state: ActionCardState = ActionCardState.PENDING,
)

enum class ActionCardState { PENDING, CONFIRMED, REJECTED }

// ── 意图路由 / 任务拆分 / 执行溯源（组 1 chat FR mock）────────────────

/** 拆分提案条目（镜像桌面 TaskDecompositionProposal.items） */
data class DecompositionItem(val title: String, val deadline: String?)

/** 任务拆分提案（镜像桌面 taskDecomposition 类型：对话流内嵌提案卡） */
data class TaskDecompositionProposal(
    val id: String,
    val taskSummary: String,
    val roleName: String,
    val roleIcon: String,
    val items: List<DecompositionItem>,
    val state: DecompositionState = DecompositionState.PENDING,
)

enum class DecompositionState { PENDING, ACCEPTED, KEPT_SINGLE }

/** 工具执行状态（FR-33 工具执行可视） */
enum class ToolStatus { RUNNING, COMPLETED, FAILED }

/**
 * 执行溯源块（FR-29，镜像桌面 ExecutionTraceBlock 三型）：
 * Think 思考 / Narration 旁白 / Action 工具动作。
 */
sealed class ExecutionTraceBlock {
    abstract val id: String

    data class Thinking(
        override val id: String,
        val content: String,
        val elapsedSeconds: Int?,
        val isActive: Boolean,
    ) : ExecutionTraceBlock()

    data class Narration(override val id: String, val content: String) : ExecutionTraceBlock()

    data class Action(
        override val id: String,
        val title: String,
        val status: ToolStatus,
        /** 桌面 actionType：shell/read/edit/write/skill/explore/tool */
        val actionType: String = "tool",
    ) : ExecutionTraceBlock()
}

// ── 主动性级别 / 活动统计（组 2 dashboard FR mock）────────────────────

/** 主动性三档（FR-12，镜像桌面 types/role.ts ProactivityLevel 与 ProactivityToggle 文案）。 */
enum class ProactivityLevel(val label: String) {
    PASSIVE("静默执行"),
    MODERATE("适度建议"),
    PROACTIVE("积极主动"),
}

/** 活动统计指标（FR-38，镜像桌面 DashboardTab metricCards 标签与顺序）。 */
enum class MetricType(val label: String) {
    TASK_TOTAL("任务总数"),
    MEMORY_COUNT("记忆数量"),
    PENDING_TASKS("待处理任务"),
    CONVERSATION_COUNT("对话数量"),
}

/** 活动统计 scope 取值（FR-38，镜像桌面 useDashboard 的 all/butler/roleId 三型）。 */
object MetricScope {
    const val ALL = "all"
    const val BUTLER = "butler"
}

/**
 * 统计时间窗（FR-38）：All=全部日期；Recent(maxDaysAgo)=最近 N 天；
 * Custom=天数前闭区间（oldest ≥ newest：起点日期更早、距今天数更大）。
 */
sealed interface ActivityWindow {
    data object All : ActivityWindow
    data class Recent(val maxDaysAgo: Int) : ActivityWindow
    data class Custom(val oldestDaysAgo: Int, val newestDaysAgo: Int) : ActivityWindow
}

// ── 记忆 / 角色涌现（组 3 role/memory FR mock）────────────────────────

/** 记忆类别（FR-8，镜像桌面 types/memory.ts MemoryCategory 与中文标签）。 */
enum class MemoryCategory(val label: String) {
    FACT("事实"),
    PREFERENCE("偏好"),
    COGNITION_UPDATE("认知模式"),
    TASK_STATUS("任务状态"),
}

/** 记忆条目（FR-8/9，镜像桌面 MemoryTab 记忆卡数据；createdAt 为 ISO 8601）。 */
data class MemoryItem(
    val id: String,
    val roleId: String,
    val category: MemoryCategory,
    val content: String,
    val createdAt: String,
)

/** 记忆来源消息（FR-8，镜像桌面 MemorySourceMessage；role 为 user/assistant）。 */
data class MemorySourceMessage(
    val id: String,
    val role: String,
    val content: String,
    val createdAt: String,
    /** 本条即记忆出处的那条消息（镜像桌面 isSource 高亮） */
    val isSource: Boolean = false,
)

/** 角色涌现提案处理态（FR-5）：待处理/已创建/已跳过。 */
enum class RoleProposalState { PENDING, CREATED, SKIPPED }

/** 角色涌现提案（FR-5，镜像桌面 RoleProposal：name/icon/color/goal，icon/color 走白名单回显）。 */
data class RoleProposal(
    val name: String,
    val icon: String?,
    val color: String?,
    val goal: String?,
    val state: RoleProposalState = RoleProposalState.PENDING,
)

/** 镜像桌面 formatMemoryTime：ISO 8601 → "yyyy/MM/dd HH:mm"（UTC 口径与桌面 getUTC* 一致）；非法输入原样返回。 */
fun formatMemoryTime(createdAt: String): String = try {
    val t = Instant.parse(createdAt).atZone(ZoneOffset.UTC)
    "%04d/%02d/%02d %02d:%02d".format(Locale.ROOT, t.year, t.monthValue, t.dayOfMonth, t.hour, t.minute)
} catch (e: DateTimeParseException) {
    createdAt
}

// ═══════════════════════════════════════════════════════════════════════
// Mock 数据（老管家语气 · 覆盖工作/家庭/学习三个角色域）
// ═══════════════════════════════════════════════════════════════════════

object SnapshotStore {

    val roles: List<RoleCard> = listOf(
        RoleCard(
            id = "role-pm",
            name = "产品经理",
            icon = "target",
            domain = RoleDomain.WORK,
            energy = 82,
            pendingCount = 3,
            lastActive = "10 分钟前",
            taskCount = 14,
            memoryCount = 38,
            sessionCount = 26,
        ),
        RoleCard(
            id = "role-father",
            name = "父亲",
            icon = "home",
            domain = RoleDomain.FAMILY,
            energy = 56,
            pendingCount = 1,
            lastActive = "昨天",
            taskCount = 6,
            memoryCount = 17,
            sessionCount = 12,
        ),
        RoleCard(
            id = "role-learner",
            name = "学习者",
            icon = "book-open",
            domain = RoleDomain.LEARN,
            energy = 33,
            pendingCount = 0,
            lastActive = "4 天前",
            taskCount = 9,
            memoryCount = 21,
            sessionCount = 8,
        ),
    )

    val tasks: List<TaskItem> = listOf(
        TaskItem(
            id = "t-1",
            title = "产品评审会议材料",
            quadrant = Quadrant.Q1,
            roleName = "产品经理",
            due = "今天 14:00",
            bigRock = false,
        ),
        TaskItem(
            id = "t-2",
            title = "回复供应商询价邮件",
            quadrant = Quadrant.Q1,
            roleName = "产品经理",
            due = "今天 17:00 前",
            bigRock = false,
        ),
        TaskItem(
            id = "t-3",
            title = "读完《深度工作》第 3 章并写笔记",
            quadrant = Quadrant.Q2,
            roleName = "学习者",
            due = "本周日",
            bigRock = true,
            // FR-24：学习者角色 4 天未活跃（≥ AT_RISK_DAYS=3），Q2 大石头被持续挤压
            protectionStatus = TaskProtectionStatus.AT_RISK,
        ),
        TaskItem(
            id = "t-4",
            title = "预约女儿的钢琴课时间",
            quadrant = Quadrant.Q2,
            roleName = "父亲",
            due = null,
            bigRock = true,
        ),
        TaskItem(
            id = "t-5",
            title = "整理季度 OKR 草稿",
            quadrant = Quadrant.Q2,
            roleName = "产品经理",
            due = "周五",
            bigRock = false,
        ),
        TaskItem(
            id = "t-6",
            title = "给母亲回电话",
            quadrant = Quadrant.Q3,
            roleName = "父亲",
            due = "今晚",
            bigRock = false,
        ),
        TaskItem(
            id = "t-7",
            title = "取干洗衣物",
            quadrant = Quadrant.Q3,
            roleName = "管家",
            due = "今天",
            bigRock = false,
        ),
        TaskItem(
            id = "t-8",
            title = "刷 20 分钟行业资讯",
            quadrant = Quadrant.Q4,
            roleName = "产品经理",
            due = null,
            bigRock = false,
        ),
    )

    val briefing: MorningBriefing = MorningBriefing(
        date = "8 月 25 日 · 周二",
        greeting = "早上好 boss。今天你的产品经理角色有个 14:00 的评审会议，材料概要已经备好；父亲角色提醒今天 16:30 是女儿的钢琴课；学习者那边昨晚沉淀了 2 条读书笔记，不着急处理。",
        sections = listOf(
            BriefingSection(
                title = "今日日程",
                body = "09:30 晨间规划 · 14:00 产品评审（材料已备） · 16:30 女儿钢琴课 · 20:00 家庭时间",
            ),
            BriefingSection(
                title = "昨夜沉淀",
                body = "学习者新增 2 条《深度工作》读书笔记；产品经理整理了评审用的竞品对比要点，已归档。",
            ),
            BriefingSection(
                title = "角色状态",
                body = "产品经理能量 82%，状态很好；父亲 56%，钢琴课相关事项需要留意；学习者 33%，有几天没读书了——今天最重要的一件事：评审前把竞品对比页过一遍。",
            ),
        ),
        actionPoints = listOf(
            BriefingActionPoint(
                id = "ap-1",
                text = "评审前 30 分钟提醒你过一遍竞品对比页",
                fromRole = "产品经理",
            ),
            BriefingActionPoint(
                id = "ap-2",
                text = "钢琴课结束后提醒你问问女儿上课感受",
                fromRole = "父亲",
            ),
        ),
    )

    val weeklyReview: WeeklyReview = WeeklyReview(
        weekLabel = "第 34 周 · 8 月 17 日 — 8 月 23 日",
        narrative = "这周你在工作和家庭两个维度都稳步推进。产品经理完成 5/7 项任务，能量从 68% 升到 82%；父亲角色的大石头「陪女儿看画展」已经完成；学习者的「读完第 3 章」还在推进中，不急。",
        roleScores = listOf(
            RoleScore(roleName = "产品经理", completed = 5, total = 7, energyNow = 82, energyDelta = +14),
            RoleScore(roleName = "父亲", completed = 3, total = 4, energyNow = 56, energyDelta = -6),
            RoleScore(roleName = "学习者", completed = 1, total = 3, energyNow = 33, energyDelta = -9),
        ),
        energyTrend = listOf(62, 58, 66, 71, 74, 70, 76),
        energyTrendDays = listOf("一", "二", "三", "四", "五", "六", "日"),
        bigRockResults = listOf(
            BigRockResult(text = "陪女儿看画展", completed = true),
            BigRockResult(text = "读完《深度工作》第 3 章", completed = false),
            BigRockResult(text = "完成季度路线图初稿", completed = true),
        ),
        closingQuestion = "下周想先关注哪个方面？我可以帮你把大石头先排进日历。",
    )

    val notices: List<NoticeItem> = listOf(
        NoticeItem(
            id = "n-knock-1",
            level = NoticeLevel.KNOCK,
            text = "boss，产品评审改到了今天下午 2 点，和接孩子的钢琴课时间撞了——需要你决定一下。",
            fromRole = "管家",
            time = "10 分钟前",
            actionable = true,
        ),
        NoticeItem(
            id = "n-knock-2",
            level = NoticeLevel.KNOCK,
            text = "供应商的报价明天到期，要不要让产品经理先把比价表整理出来？",
            fromRole = "产品经理",
            time = "2 小时前",
            actionable = true,
            read = true,
        ),
        // FR-24 Q2 保护提醒（母本 q2_protection_reminder.rs:122-135：轻触级通知、文案逐字镜像，
        // 与 at_risk 任务 t-3 对应）
        NoticeItem(
            id = "n-tap-3",
            level = NoticeLevel.TAP,
            text = "你的'读完《深度工作》第 3 章并写笔记'任务已经 3 天没动了，要不要今天安排一下？",
            fromRole = "学习者",
            time = "1 小时前",
        ),
        NoticeItem(
            id = "n-tap-1",
            level = NoticeLevel.TAP,
            text = "顺便说一句，父亲角色提醒这周五是女儿钢琴课。",
            fromRole = "父亲",
            time = "3 小时前",
        ),
        NoticeItem(
            id = "n-tap-2",
            level = NoticeLevel.TAP,
            text = "学习者昨晚沉淀了 2 条读书笔记，已归档。",
            fromRole = "学习者",
            time = "昨天 23:40",
            read = true,
        ),
        NoticeItem(
            id = "n-whisper-1",
            level = NoticeLevel.WHISPER,
            text = "产品经理整理了 3 条竞品对比要点。",
            fromRole = "产品经理",
            time = "昨天",
        ),
        NoticeItem(
            id = "n-whisper-2",
            level = NoticeLevel.WHISPER,
            text = "父亲角色更新了家庭日历（女儿钢琴课改期）。",
            fromRole = "父亲",
            time = "昨天",
            read = true,
        ),
    )

    /** 管家开场对话（配对成功后初始消息流）。 */
    val initialChat: List<ChatMessage> = listOf(
        ChatMessage(
            id = "m-1",
            fromButler = true,
            text = "早上好 boss。今天最重要的一件事是下午 2 点的产品评审，材料产品经理已经备好了。另外，女儿钢琴课在四点半，我提前提醒您。",
        ),
    )

    /** 用户发言后管家的流式回复轮换（mock 打字机效果）。 */
    val butlerReplies: List<String> = listOf(
        "好的，这事我记下了。要不要让产品经理跟进？完成后第一时间告诉你。",
        "明白。这件事我建议放进 Q2——重要，但不必现在动手。您看呢？",
        "已经为您记下来了。顺带一提，今天 Q1 还有两件事，需要我复述一下吗？",
        "收到，boss。学习者角色这几天能量偏低（33%），要不要安排 20 分钟的读书时间？",
    )

    // ── 组 1 chat 增补 mock ─────────────────────────────────────────────

    /** 委派路由表（FR-1）：关键词 → 目标角色。消息含关键词即走两段委派。 */
    val delegationKeywords: Map<String, String> = mapOf(
        "产品经理" to "role-pm",
        "父亲" to "role-father",
        "学习者" to "role-learner",
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

    /** 角色会话种子（FR-20：切到角色视图时的初始消息流）。 */
    val roleChatSeeds: Map<String, List<ChatMessage>> = mapOf(
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

    /** 工具执行阶段序列（FR-33：流式期间的工具名+状态指示轮换）。 */
    val toolExecutionStages: List<String> = listOf(
        "读取日程与任务快照",
        "整理竞品对比要点",
        "生成回复草稿",
    )

    // ── 组 2 dashboard 增补 mock ────────────────────────────────────────

    /**
     * 活动分桶账本（FR-38 mock）：角色 id（含 "butler"）→ MetricType → 4 个天数桶计数。
     * 桶边界：[0]=0–2 天前（近3天）、[1]=3–6 天前（近7天段）、[2]=7–29 天前（近1月段）、[3]=30+ 天前。
     * 「全部日期」= 四桶总和，与角色卡统计数字对齐（如产品经理任务 Σ=14）。
     * 接真实连接层后由 SNAPSHOT 帧的聚合指标替换。
     */
    val activityLedger: Map<String, Map<MetricType, List<Int>>> = mapOf(
        "butler" to mapOf(
            MetricType.TASK_TOTAL to listOf(1, 1, 0, 0),
            MetricType.MEMORY_COUNT to listOf(0, 1, 2, 0),
            MetricType.PENDING_TASKS to listOf(0, 0, 0, 0),
            MetricType.CONVERSATION_COUNT to listOf(5, 4, 2, 1),
        ),
        "role-pm" to mapOf(
            MetricType.TASK_TOTAL to listOf(4, 3, 5, 2),
            MetricType.MEMORY_COUNT to listOf(6, 10, 14, 8),
            MetricType.PENDING_TASKS to listOf(2, 1, 0, 0),
            MetricType.CONVERSATION_COUNT to listOf(8, 10, 6, 2),
        ),
        "role-father" to mapOf(
            MetricType.TASK_TOTAL to listOf(2, 1, 2, 1),
            MetricType.MEMORY_COUNT to listOf(4, 6, 5, 2),
            MetricType.PENDING_TASKS to listOf(1, 0, 0, 0),
            MetricType.CONVERSATION_COUNT to listOf(4, 5, 2, 1),
        ),
        "role-learner" to mapOf(
            MetricType.TASK_TOTAL to listOf(1, 2, 4, 2),
            MetricType.MEMORY_COUNT to listOf(2, 5, 8, 6),
            MetricType.PENDING_TASKS to listOf(0, 0, 0, 0),
            MetricType.CONVERSATION_COUNT to listOf(1, 3, 3, 1),
        ),
    )

    /** 天数桶边界（daysAgo 闭区间），顺序与 activityLedger 桶注释一一对应。 */
    private val ACTIVITY_BUCKET_BOUNDS: List<IntRange> =
        listOf(0..2, 3..6, 7..29, 30..Int.MAX_VALUE)

    /**
     * 按 scope+时间窗聚合活动指标（FR-38）。
     * scopeId："all"=全部 agent 合计（含管家）；否则角色 id（含 "butler"）。
     * 自定义窗口按「桶重叠即整桶计入」近似（原型粒度降级，不逐日精确）。
     */
    fun activityMetrics(scopeId: String, window: ActivityWindow): Map<MetricType, Int> {
        val ledgers = if (scopeId == MetricScope.ALL) {
            activityLedger.values
        } else {
            listOfNotNull(activityLedger[scopeId])
        }
        return MetricType.entries.associateWith { type ->
            ledgers.sumOf { ledger ->
                val buckets = ledger[type].orEmpty()
                buckets.indices.sumOf { i ->
                    if (windowOverlapsBucket(window, ACTIVITY_BUCKET_BOUNDS[i])) buckets[i] else 0
                }
            }
        }
    }

    /** 区间重叠判定：窗口在 daysAgo 轴上为 [newestDaysAgo, oldestDaysAgo]，桶为 bounds。 */
    private fun windowOverlapsBucket(window: ActivityWindow, bounds: IntRange): Boolean = when (window) {
        ActivityWindow.All -> true
        is ActivityWindow.Recent -> bounds.first <= window.maxDaysAgo
        is ActivityWindow.Custom ->
            bounds.first <= window.oldestDaysAgo && bounds.last >= window.newestDaysAgo
    }

    // ── 组 3 role/memory 增补 mock ──────────────────────────────────────

    /**
     * 角色记忆（FR-8/9）：每角色 3 条可见记忆（fact/preference/cognition_update），
     * PM 额外一条 task_status 用于锁定「永不显示」契约（桌面 visibleMemories 过滤）。
     */
    val memories: List<MemoryItem> = listOf(
        MemoryItem("mem-pm-1", "role-pm", MemoryCategory.FACT, "boss 的产品评审固定在周二下午 2 点，材料需要提前一天备好。", "2026-08-24T09:12:00Z"),
        MemoryItem("mem-pm-2", "role-pm", MemoryCategory.PREFERENCE, "boss 倾向用竞品对比表而不是长文档过评审材料。", "2026-08-22T15:40:00Z"),
        MemoryItem("mem-pm-3", "role-pm", MemoryCategory.COGNITION_UPDATE, "boss 在 OKR 上更看重方向共识，而非精确数字。", "2026-08-20T11:05:00Z"),
        MemoryItem("mem-pm-4", "role-pm", MemoryCategory.TASK_STATUS, "季度 OKR 草稿已拆分为 3 个任务推进中。", "2026-08-25T08:30:00Z"),
        MemoryItem("mem-fa-1", "role-father", MemoryCategory.FACT, "女儿的钢琴课在周二 16:30，老师姓陈。", "2026-08-23T18:20:00Z"),
        MemoryItem("mem-fa-2", "role-father", MemoryCategory.PREFERENCE, "boss 希望家庭事务避开周末上午的家庭时间。", "2026-08-21T20:15:00Z"),
        MemoryItem("mem-fa-3", "role-father", MemoryCategory.COGNITION_UPDATE, "boss 更愿意亲自到场陪女儿，而不是只安排时间。", "2026-08-18T21:00:00Z"),
        MemoryItem("mem-le-1", "role-learner", MemoryCategory.FACT, "boss 在读《深度工作》，进度在第 3 章中段。", "2026-08-24T22:45:00Z"),
        MemoryItem("mem-le-2", "role-learner", MemoryCategory.PREFERENCE, "boss 喜欢在晚上 9 点后写读书笔记，每次 20 分钟。", "2026-08-19T22:10:00Z"),
    )

    /**
     * 记忆来源消息（FR-8）：memoryId → 来源对话（首条 isSource=true 镜像桌面高亮）。
     * mem-le-2 故意缺席 → 演示「来源对话已不可用」分支（桌面三分支归一为该文案）。
     * mem-pm-4（task_status）永不被查询（UI 不显示该类别）。
     */
    val memorySources: Map<String, List<MemorySourceMessage>> = mapOf(
        "mem-pm-1" to listOf(
            MemorySourceMessage("src-pm-1-u", "user", "下周的产品评审还是周二下午吗？我怕材料来不及准备。", "2026-08-24T09:10:00Z"),
            MemorySourceMessage("src-pm-1-a", "assistant", "评审固定在周二下午 2 点，材料我会提前一天备好，您不用惦记。", "2026-08-24T09:12:00Z", isSource = true),
        ),
        "mem-pm-2" to listOf(
            MemorySourceMessage("src-pm-2-u", "user", "评审材料我想直接看竞品对比，别发长文档。", "2026-08-22T15:38:00Z"),
            MemorySourceMessage("src-pm-2-a", "assistant", "好的，我按竞品对比表准备，重点标出差异点。", "2026-08-22T15:40:00Z", isSource = true),
        ),
        "mem-pm-3" to listOf(
            MemorySourceMessage("src-pm-3-u", "user", "这季度 OKR 我想先对齐方向，数字后面再调。", "2026-08-20T11:03:00Z"),
            MemorySourceMessage("src-pm-3-a", "assistant", "明白，方向共识优先，我先出三个候选方向给您圈选。", "2026-08-20T11:05:00Z", isSource = true),
        ),
        "mem-fa-1" to listOf(
            MemorySourceMessage("src-fa-1-u", "user", "女儿钢琴课这周还是周二吗？", "2026-08-23T18:18:00Z"),
            MemorySourceMessage("src-fa-1-a", "assistant", "是的，周二 16:30，陈老师那边已经确认过了。", "2026-08-23T18:20:00Z", isSource = true),
        ),
        "mem-fa-2" to listOf(
            MemorySourceMessage("src-fa-2-u", "user", "家里的事尽量别安排在周末上午，那是我们的家庭时间。", "2026-08-21T20:13:00Z"),
            MemorySourceMessage("src-fa-2-a", "assistant", "记下了，周末上午留给家庭，其他事项我都避开这个时段。", "2026-08-21T20:15:00Z", isSource = true),
        ),
        "mem-fa-3" to listOf(
            MemorySourceMessage("src-fa-3-u", "user", "女儿的画展我想亲自陪她去，不用安排别人。", "2026-08-18T20:58:00Z"),
            MemorySourceMessage("src-fa-3-a", "assistant", "明白，这类事您都亲自到场，我只负责提醒和买票。", "2026-08-18T21:00:00Z", isSource = true),
        ),
        "mem-le-1" to listOf(
            MemorySourceMessage("src-le-1-u", "user", "《深度工作》读到第 3 章了，有点慢。", "2026-08-24T22:43:00Z"),
            MemorySourceMessage("src-le-1-a", "assistant", "不着急，您每晚都有固定的读书时间，按这个节奏这周能读完第 3 章。", "2026-08-24T22:45:00Z", isSource = true),
        ),
    )

    /** 按角色查记忆（FR-8）：镜像桌面 useMemories({roleId}) 的过滤语义。 */
    fun memoriesOf(roleId: String): List<MemoryItem> = memories.filter { it.roleId == roleId }

    /** 角色涌现提案种子（FR-5，mock 触发：管家视图第 2 轮回复后浮现提案卡）。 */
    val roleProposal: RoleProposal = RoleProposal(
        name = "策划师",
        icon = "lightbulb",
        color = "#8B5CF6",
        goal = "把零散的想法收敛成可执行的方案",
    )

    // ── 组 5 review 增补 mock ──────────────────────────────────────────

    /**
     * 大石头建议（FR-17）：角色 id → 建议文案（镜像桌面 review_get_bigrock_suggestions）。
     * 学习者故意缺席 → 演示「手动填写本周大石头」分支（桌面 suggestions 为空时的占位）。
     */
    val bigRockSuggestions: Map<String, List<String>> = mapOf(
        "role-pm" to listOf(
            "完成下季度路线图评审并锁定排期",
            "整理一次完整的客户反馈复盘",
        ),
        "role-father" to listOf(
            "安排一次全家的周末户外活动",
            "陪女儿完成一幅拼图作品",
        ),
    )

    // ── 组 6 onboarding 增补 mock ──────────────────────────────────────

    /** 引导开场问候（FR-21，镜像桌面 startOnboarding 失败 fallback 文案）。 */
    val onboardingGreeting: String =
        "你好，我是你的数字分身管家。很高兴为你服务！聊聊你最近在忙什么？"

    /** 五步访谈输入 placeholder（FR-21，镜像桌面 OnboardingView placeholders 逐字）。 */
    val onboardingPlaceholders: List<String> = listOf(
        "输入你的名字或昵称...",
        "比如：我是一个产品经理，最近在忙新产品上线...",
        "聊聊你最关注的方面...",
        "确认，或者告诉我你想调整...",
        "好的，创建吧！",
    )

    /** 五步访谈管家回复（mock；桌面由 LLM 流式生成）。 */
    val onboardingReplies: List<String> = listOf(
        "很高兴认识你。我平时会帮你把工作、家庭、学习这些方面都照顾到——先从你最常忙的事说起？",
        "听起来这是个重要的方向。要不要我帮你盯着这类事？到了合适的时机我会主动提醒你。",
        "明白了。看来这方面值得专门立一个角色来跟进，稍后我可能会有个提议。",
        "好的，都记下了。我会把这些偏好放进你的个人画像，之后的服务都会以此为基准。",
        "那就这么定了。角色建好之后，任务、记忆和简报都会围绕它展开。有任何事随时叫我。",
    )
}
