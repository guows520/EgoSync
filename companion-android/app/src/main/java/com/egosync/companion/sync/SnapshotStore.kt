package com.egosync.companion.sync

import com.egosync.companion.connection.CompanionLog
import com.egosync.companion.connection.OfflineSnapshotInfo
import com.egosync.companion.ui.theme.Quadrant
import com.egosync.companion.ui.theme.RoleDomain
import java.io.IOException
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset
import java.time.format.DateTimeParseException
import java.util.Locale
import kotlin.coroutines.CoroutineContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

/**
 * 快照数据模型 + 帧驱动快照存储（Story 13.2 换装）。
 *
 * 对应架构「快照引擎」：手机端对快照只读渲染（桌面为唯一事实源）。
 * [SnapshotStore] 为实例类（AppModelContainer 唯一换装点装配）：SNAPSHOT /
 * STATE_DELTA 帧经 [SnapshotFrameHandler] 重组解析后驱动 [StateMerger] 全量替换，
 * 上层 ViewModel 经既有取数接口（roles/tasks/briefing/weeklyReview/notices/
 * conversationsOf/memoriesOf/activityMetrics）取数，UI 层零改动。
 *
 * 负向契约（AC5 防呆）：本类**不携带任何 mock 数据**；记忆条目内容不在快照
 * 通道（memoriesOf 恒空，记忆页待 13.3 指令通道现查）；演示性数据（对话流/
 * 建议/引导）迁往各自 ui 包私有文件。
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
    /** 快照无 per-role 记忆数（桌面仅全局 metrics.memoryCount）——null → UI 显示「—」（NFR-M3 诚实降级）。 */
    val memoryCount: Int?,
    val sessionCount: Int,
)

// ── 四象限任务 ─────────────────────────────────────────────────────────

/** Q2 保护状态（FR-24，镜像桌面 types/task.ts:2 TaskProtectionStatus）。 */
enum class TaskProtectionStatus { NORMAL, AT_RISK }

/** 任务归属（镜像桌面 types/task.ts:8 TaskOwnerType）。 */
enum class TaskOwner { BUTLER, ROLE }

/** 归属筛选稳定键：管家固定 "butler"（镜像桌面 TaskOverviewTab.tsx:34-36 ownerKey）。 */
const val TASK_OWNER_BUTLER_KEY = "butler"

data class TaskItem(
    val id: String,
    val title: String,
    val quadrant: Quadrant,
    val roleName: String,
    /** 结构化归属（镜像桌面 types/task.ts:12-13）：ownerType + roleId 是归属筛选的稳定标识；roleName 仅为展示字符串。 */
    val ownerType: TaskOwner,
    val roleId: String?,
    val due: String?,
    /** 大石头：本周不可妥协的优先项（每角色每周 1~2 件） */
    val bigRock: Boolean,
    val done: Boolean = false,
    /** Q2 保护提醒（FR-24）：at_risk = 重要任务被持续挤压（母本 q2_protection_reminder.rs 检测）。 */
    val protectionStatus: TaskProtectionStatus = TaskProtectionStatus.NORMAL,
)

/** 归属筛选键（镜像桌面 ownerKey）：管家恒为 "butler"，角色取 roleId，缺失归 "unknown"。 */
fun TaskItem.ownerKey(): String =
    if (ownerType == TaskOwner.BUTLER) TASK_OWNER_BUTLER_KEY else roleId ?: "unknown"

/**
 * 新建任务输入（镜像桌面 types/task.ts:44-52 CreateTaskInput 的移动端子集）：
 * quadrant = null 表示「智能判断」——不指定象限，由后台异步分类（原型以 classifying 过渡态模拟）。
 */
data class CreateTaskInput(
    val title: String,
    val ownerType: TaskOwner,
    val roleId: String? = null,
    val quadrant: Quadrant? = null,
    val due: String? = null,
    val bigRock: Boolean = false,
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
    /** 近 7 天能量趋势（聚合值 0~100），供 Canvas 柱状图自绘。13.2 起为空（桌面无历史序列，UI 置诚实占位）。 */
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
    /** T-S10 离线待发态（!commandReady 入队）：网络恢复后自动发送 */
    val pending: Boolean = false,
)

/**
 * 会话（多会话，镜像桌面 Conversation 类型）：
 * id 唯一；title 由首条用户消息生成（空=尚未命名，列表显示「新对话」）；
 * updatedAt 为毫秒时间戳，驱动相对时间显示与列表排序（最新在前）。
 */
data class ChatConversation(
    val id: String,
    val title: String,
    val updatedAt: Long,
    val messages: List<ChatMessage> = emptyList(),
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
 * Custom=绝对日期闭区间（**携带绝对日期而非相对天数**——相对天数跨午夜后会
 * 前移，标签与实际窗口错位；13.2 T8 崩溃守卫整改）。oldest ≤ newest。
 */
sealed interface ActivityWindow {
    data object All : ActivityWindow
    data class Recent(val maxDaysAgo: Int) : ActivityWindow
    data class Custom(val oldestDate: LocalDate, val newestDate: LocalDate) : ActivityWindow
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
// 帧驱动快照存储（实例类，AppModelContainer 唯一换装点装配）
// ═══════════════════════════════════════════════════════════════════════

/**
 * 帧驱动快照存储：
 * - [applySnapshot] 由 [SnapshotFrameHandler]（SNAPSHOT/STATE_DELTA 全量替换）调用；
 * - 快照缓存（[SnapshotCacheFile]）异步落盘，启动即加载（离线呈现最后已知快照）；
 * - 既有取数接口字段名与签名不变，值由 [SnapshotMapper] 从当前快照派生。
 */
class SnapshotStore(
    private val cache: SnapshotCacheFile? = null,
    private val scope: CoroutineScope? = null,
    private val ioDispatcher: CoroutineContext = Dispatchers.IO,
    private val nowProvider: () -> Long = System::currentTimeMillis,
) {

    private val merger = StateMerger()

    /** 内存态与封存标志的互斥（clear 与在飞帧 apply 的顺序化，13.2 评审 P4）。 */
    private val lock = Any()

    /** 磁盘操作单飞（save/clear/启动加载互斥，13.2 评审 P5：并发写互踩/密钥别名竞态）。 */
    private val diskMutex = Mutex()

    /**
     * 解除配对封存（13.2 评审 P4）：[clear] 置位后在飞帧/在途落盘/迟到缓存一律
     * 拒绝，防解配后数据复活；新会话建立（换装点 frameConsumerFactory）经 [resume] 解封。
     */
    @Volatile private var sealed = false

    /** 快照态（含元数据）：上层经此订阅 STATE_DELTA 即时刷新（AC2）。 */
    val state: StateFlow<SnapshotState> get() = merger.state

    init {
        // 启动读缓存：离线时呈现最后已知快照；在线则首个 SNAPSHOT 全量覆盖（AC3）
        val cacheFile = cache
        if (cacheFile != null && scope != null) {
            scope.launch(ioDispatcher) { loadCached(cacheFile) }
        }
    }

    /** SNAPSHOT/STATE_DELTA 全量替换；rawJson 提供时异步落盘缓存。 */
    fun applySnapshot(snapshot: DesktopSnapshot, rawJson: ByteArray? = null) {
        val json: ByteArray? = synchronized(lock) {
            if (sealed) {
                // 解配封存：迟到帧不得复活已清数据（评审 P4）
                CompanionLog.warn("Companion/Snapshot", "已解配，丢弃迟到快照帧")
                return
            }
            merger.applySnapshot(snapshot)
            rawJson
        }
        val cacheFile = cache
        if (json == null || cacheFile == null || scope == null) return
        scope.launch(ioDispatcher) {
            diskMutex.withLock {
                // 封存检查在锁内：clear 与在途落盘的次序无论谁先，缓存终态一致
                if (sealed) return@withLock
                try {
                    cacheFile.save(json)
                } catch (e: Exception) {
                    // 缓存失败不阻塞渲染：下次快照会重写（自愈）
                    CompanionLog.warn("Companion/Snapshot", "快照缓存写入失败: ${e.javaClass.simpleName}")
                }
            }
        }
    }

    private suspend fun loadCached(cacheFile: SnapshotCacheFile) = diskMutex.withLock {
        val bytes = try {
            cacheFile.load()
        } catch (e: SnapshotCacheException) {
            // 未知版本：显式失败不静默（T4 单测锁定）——等待在线 SNAPSHOT 覆盖
            CompanionLog.warn("Companion/Snapshot", "快照缓存版本不可读，忽略: ${e.message}")
            null
        } catch (e: IOException) {
            // 读缓存期间文件被并发清除/磁盘 IO 错误（exists 与 read 之间的窗口）：
            // 缓存不可用不得让启动协程崩溃（评审 P2）
            CompanionLog.warn("Companion/Snapshot", "快照缓存读取失败，忽略: ${e.javaClass.simpleName}")
            null
        } ?: return@withLock // 无缓存或已自愈删除
        val snapshot = try {
            SnapshotParser.parse(bytes.decodeToString(throwOnInvalidSequence = false))
        } catch (e: SnapshotFormatException) {
            CompanionLog.warn("Companion/Snapshot", "快照缓存解析失败，忽略")
            null
        } ?: return@withLock
        synchronized(lock) {
            if (sealed) return@withLock // 启动加载与 unpair 竞态：封存后整条丢弃
            // 评审 P1：在线快照已就位（缓存读协程迟到）则陈旧缓存不得覆盖
            if (merger.state.value.snapshot != null) {
                CompanionLog.info("Companion/Snapshot", "在线快照已就位，跳过陈旧缓存")
                return@withLock
            }
            merger.applySnapshot(snapshot)
        }
        CompanionLog.info("Companion/Snapshot", "快照缓存已加载（离线只读呈现）")
    }

    // ── 既有取数接口（快照派生，签名不变）────────────────────────────

    /** 角色卡（含任务/会话统计；记忆格 null → 「—」）。 */
    val roles: List<RoleCard>
        get() = state.value.snapshot?.let { SnapshotMapper.toRoleCards(it, nowProvider()) } ?: emptyList()

    /** 四象限任务。 */
    val tasks: List<TaskItem>
        get() = state.value.snapshot?.let(SnapshotMapper::toTaskItems) ?: emptyList()

    /** 晨间简报（最新一条；无数据为空态）。 */
    val briefing: MorningBriefing
        get() = state.value.snapshot?.let(SnapshotMapper::toBriefing) ?: EMPTY_BRIEFING

    /** 周复盘（最新一条；无数据为空态）。 */
    val weeklyReview: WeeklyReview
        get() = state.value.snapshot?.let { SnapshotMapper.toWeeklyReview(it, nowProvider()) } ?: EMPTY_REVIEW

    /**
     * 未读通知（快照 notifications 域只含未读）。已读状态由通知适配器层以本地
     * 已读记录叠加（13.2 评审决策 A）：STATE_DELTA 全量重发不再把已读冲回未读。
     */
    val notices: List<NoticeItem>
        get() = state.value.snapshot?.let { SnapshotMapper.toNotices(it, nowProvider()) } ?: emptyList()

    /** 某视图（null=管家）的会话列表（快照 conversations 域按 roleId 归属派生）。 */
    fun conversationsOf(roleId: String?): List<ChatConversation> {
        val snapshot = state.value.snapshot ?: return emptyList()
        return SnapshotMapper.toConversationsByView(snapshot)[roleId].orEmpty()
    }

    /** 记忆条目：快照通道不含记忆内容（AC5），恒为空——记忆页待 13.3 指令通道现查。 */
    fun memoriesOf(@Suppress("UNUSED_PARAMETER") roleId: String): List<MemoryItem> = emptyList()

    /**
     * 活动统计（FR-38 快照全量口径）：all=桌面全局指标；
     * butler/角色=按归属聚合；记忆数仅 all 有真值，其余 null（「—」）。
     * 无快照（冷启动且无缓存）时全指标为 null（「—」）。
     *
     * [window] 时间窗参数当前**不生效**（快照 metrics 域仅聚合数、无时间明细；
     * Mapper 注释同源）——签名按 T2 冻结保留；真正筛选待桌面 schema 扩展
     * （13.2 评审决策 C：另立 story 扩展桌面 metrics schema）。
     */
    fun activityMetrics(scopeId: String, window: ActivityWindow): Map<MetricType, Int?> =
        state.value.snapshot?.let { SnapshotMapper.activityMetrics(it, scopeId, window) }
            ?: MetricType.entries.associateWith { null }

    /** 降级态信息（AC3/FR-40）：缓存可用性与数据截止时间，供 TransportStatus.Degraded 呈现。 */
    fun offlineInfo(): OfflineSnapshotInfo {
        val snapshot = state.value.snapshot
            ?: return OfflineSnapshotInfo(snapshotAvailable = false, dataAsOf = null)
        // 评审 P13：格式化为用户可读本地时间——降级横幅直出原始 ISO 串是可用性缺陷
        return OfflineSnapshotInfo(
            snapshotAvailable = true,
            dataAsOf = SnapshotMapper.formatDataCutoff(snapshot.dataCutoffAt ?: snapshot.generatedAt),
        )
    }

    /** 解除配对配套：封存通道 + 清空内存态并异步清除快照缓存（信任锚清除之外的数据清除；桌面端数据不受影响）。 */
    fun clear() {
        synchronized(lock) {
            sealed = true
            merger.reset()
        }
        val cacheFile = cache ?: return
        scope?.launch(ioDispatcher) {
            diskMutex.withLock {
                try {
                    cacheFile.clear()
                } catch (e: Exception) {
                    CompanionLog.warn("Companion/Snapshot", "快照缓存清除失败: ${e.javaClass.simpleName}")
                }
            }
        }
    }

    /** 解封快照通道（新会话建立时由换装点调用）：重新配对后允许接收数据。 */
    fun resume() {
        if (sealed) {
            CompanionLog.info("Companion/Snapshot", "快照通道解封（新会话建立）")
        }
        sealed = false
    }

    private companion object {
        val EMPTY_BRIEFING = MorningBriefing(
            date = "", greeting = "", sections = emptyList(), actionPoints = emptyList(),
        )

        val EMPTY_REVIEW = WeeklyReview(
            weekLabel = "", narrative = "", roleScores = emptyList(),
            energyTrend = emptyList(), energyTrendDays = emptyList(),
            bigRockResults = emptyList(),
            closingQuestion = "下周想先关注哪个方面？我可以帮你把大石头先排进日历。",
        )
    }
}
