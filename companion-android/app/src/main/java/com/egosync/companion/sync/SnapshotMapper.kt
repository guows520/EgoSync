package com.egosync.companion.sync

import com.egosync.companion.ui.theme.Quadrant
import com.egosync.companion.ui.theme.RoleDomain
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeParseException
import java.time.temporal.WeekFields
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject
import java.util.Locale

/**
 * 快照 → 既有 UI 模型派生（Story 13.2 换装的映射层）。
 *
 * 关键决策（story Dev Notes §5）：
 * - 角色卡记忆格：快照无 per-role 记忆数（桌面仅全局 metrics.memoryCount）——
 *   **memoryCount 返回 null（UI 显示"—"）**，禁止为凑数显示全局数或假数字（NFR-M3）。
 * - 活动统计：任务/待办/会话数从 tasks/conversations 按 roleId 聚合；记忆数仅
 *   all-scope 有全局真值，其余 scope 为 null；时间窗筛选呈现快照全量口径
 *   （FR-38 筛选交互走后续 schemaVersion 演进）。
 * - 能量趋势：桌面 energyTrends 为各角色**当前**能量快照 JSON（非时间序列）——
 *   解析镜像桌面 useWeeklyReview（非法 JSON 降级 null），趋势柱状数据返回空
 *   （UI 置诚实占位，禁止假 7 点数据冒充趋势）。
 * - 记忆条目内容永不进快照（AC5 防呆）。
 */
object SnapshotMapper {

    // ── 角色卡 ────────────────────────────────────────────────────────

    fun toRoleCards(snapshot: DesktopSnapshot, nowMs: Long): List<RoleCard> =
        snapshot.dashboard.statuses.map { status ->
            RoleCard(
                id = status.roleId,
                name = status.roleName,
                icon = status.roleIcon,
                domain = domainForColor(status.roleColor),
                energy = status.energy,
                pendingCount = status.pendingTasksCount.toInt(),
                lastActive = formatRelativeTimeIso(status.lastActiveAt, nowMs),
                taskCount = snapshot.tasks.count { it.roleId == status.roleId },
                memoryCount = null, // 快照无 per-role 记忆数（诚实降级"—"）
                sessionCount = snapshot.conversations.count { it.roleId == status.roleId },
            )
        }

    /**
     * 角色自带色 → 色温域（UX-M2：UI 层零改动，域枚举保留为色温分类位）：
     * 按真实 roleColor 的色相分桶——绿→健康 / 青蓝→工作 / 紫→学习 / 暖色→家庭。
     * 无效色默认工作（冷色）。
     */
    fun domainForColor(colorHex: String): RoleDomain {
        val hue = hueOf(colorHex) ?: return RoleDomain.WORK
        return when {
            hue >= 70f && hue < 170f -> RoleDomain.HEALTH
            hue >= 170f && hue < 260f -> RoleDomain.WORK
            hue >= 260f && hue < 320f -> RoleDomain.LEARN
            else -> RoleDomain.FAMILY
        }
    }

    private fun hueOf(colorHex: String): Float? {
        val hex = colorHex.removePrefix("#")
        if (hex.length != 6) return null
        val rgb = hex.toIntOrNull(16) ?: return null
        val r = ((rgb shr 16) and 0xFF) / 255f
        val g = ((rgb shr 8) and 0xFF) / 255f
        val b = (rgb and 0xFF) / 255f
        val max = maxOf(r, g, b)
        val min = minOf(r, g, b)
        if (max == min) return null // 灰色无色相
        val delta = max - min
        return when (max) {
            r -> 60f * (((g - b) / delta).mod(6f))
            g -> 60f * ((b - r) / delta + 2f)
            else -> 60f * ((r - g) / delta + 4f)
        }
    }

    // ── 任务 ──────────────────────────────────────────────────────────

    fun toTaskItems(snapshot: DesktopSnapshot): List<TaskItem> = snapshot.tasks.map { task ->
        TaskItem(
            id = task.id,
            title = task.title,
            quadrant = Quadrant.entries.firstOrNull { it.code == task.quadrant } ?: Quadrant.Q3,
            roleName = task.roleName ?: if (task.ownerType == "butler") "管家" else "未知角色",
            ownerType = if (task.ownerType == "butler") TaskOwner.BUTLER else TaskOwner.ROLE,
            roleId = task.roleId,
            due = task.deadline,
            bigRock = task.isBigRock,
            done = task.isCompleted,
            protectionStatus =
                if (task.protectionStatus == "at_risk") TaskProtectionStatus.AT_RISK
                else TaskProtectionStatus.NORMAL,
        )
    }

    // ── 晨间简报（最新一条，按 date）─────────────────────────────────

    /** 无简报时的空态（UI 各分区渲染为空，不造假内容）。 */
    private val EMPTY_BRIEFING = MorningBriefing(
        date = "", greeting = "", sections = emptyList(), actionPoints = emptyList(),
    )

    /** 简报问候段为 UI 文案（非数据）；行动点属生成性内容（指令通道 13.3），快照口径无。 */
    fun toBriefing(snapshot: DesktopSnapshot): MorningBriefing {
        val latest = snapshot.briefings.maxByOrNull { it.date } ?: return EMPTY_BRIEFING
        return MorningBriefing(
            date = formatDateLabel(latest.date),
            greeting = "早上好。以下是来自桌面的晨间简报。",
            sections = listOf(BriefingSection(title = "今日简报", body = latest.content)),
            actionPoints = emptyList(),
        )
    }

    // ── 周复盘（最新一条，按 weekStart）───────────────────────────────

    private val EMPTY_REVIEW = WeeklyReview(
        weekLabel = "", narrative = "", roleScores = emptyList(),
        energyTrend = emptyList(), energyTrendDays = emptyList(),
        bigRockResults = emptyList(),
        closingQuestion = CLOSING_QUESTION,
    )

    /** 收尾问句为管家 UI 文案（非数据），与 mock 原文一致。 */
    private const val CLOSING_QUESTION = "下周想先关注哪个方面？我可以帮你把大石头先排进日历。"

    fun toWeeklyReview(snapshot: DesktopSnapshot, nowMs: Long): WeeklyReview {
        val latest = snapshot.weeklyReviews.maxByOrNull { it.weekStart } ?: return EMPTY_REVIEW
        val energyByRole = parseEnergyTrends(latest.energyTrends)
        return WeeklyReview(
            weekLabel = formatWeekLabel(latest.weekStart, latest.weekEnd),
            narrative = latest.summary,
            roleScores = snapshot.roles.map { role ->
                RoleScore(
                    roleName = role.name,
                    completed = snapshot.tasks.count { it.roleId == role.id && it.isCompleted },
                    total = snapshot.tasks.count { it.roleId == role.id },
                    energyNow = energyByRole?.get(role.id) ?: role.energy,
                    energyDelta = 0, // 快照无历史序列，增量为 0（中性显示）
                )
            },
            // 能量趋势无真实数据源（桌面 energyTrends 为当前能量快照，非时间序列）：
            // 返回空，UI 置诚实占位（§5 裁决，禁止假 7 点数据冒充趋势）
            energyTrend = emptyList(),
            energyTrendDays = emptyList(),
            bigRockResults = parseBigRockResults(latest.bigrockStatus),
            closingQuestion = CLOSING_QUESTION,
        )
    }

    /**
     * 解析 energyTrends JSON（镜像桌面 useWeeklyReview：`{"<roleId>": {"energy": Int}}`，
     * 各角色当前能量快照）。非法 JSON 降级 null（镜像桌面 catch → null）。
     */
    fun parseEnergyTrends(raw: String): Map<String, Int>? = try {
        val json = JSONObject(raw)
        buildMap {
            for (key in json.keys()) {
                put(key, json.getJSONObject(key).getInt("energy"))
            }
        }
    } catch (e: JSONException) {
        null
    }

    /** 解析 bigrockStatus JSON（`[{id,title,isCompleted,completedAt,roleName}]`）；非法降级空。 */
    fun parseBigRockResults(raw: String): List<BigRockResult> = try {
        val array = JSONArray(raw)
        List(array.length()) { i ->
            val item = array.getJSONObject(i)
            BigRockResult(
                text = item.getString("title"),
                completed = item.getBoolean("isCompleted"),
            )
        }
    } catch (e: JSONException) {
        emptyList()
    }

    // ── 通知（未读域）─────────────────────────────────────────────────

    fun toNotices(snapshot: DesktopSnapshot, nowMs: Long): List<NoticeItem> =
        snapshot.notifications.map { notification ->
            NoticeItem(
                id = notification.id,
                level = when (notification.level) {
                    "knock" -> NoticeLevel.KNOCK
                    "tap" -> NoticeLevel.TAP
                    else -> NoticeLevel.WHISPER
                },
                text = notification.content,
                fromRole = notification.roleName,
                time = formatRelativeTimeIso(notification.createdAt, nowMs),
                // 敲门级通知附待决策操作（respond 属指令通道 13.3，13.2 仅呈现）
                actionable = notification.level == "knock",
            )
        }

    // ── 会话（多会话列表 + 历史消息）─────────────────────────────────

    /**
     * 快照会话域 → 按视图（null=管家）分组的会话列表。roleId 为 null 的会话归
     * 管家视图；角色会话的 assistant 消息带 senderRoleId（FR-1 委派第二段同语义）。
     */
    fun toConversationsByView(snapshot: DesktopSnapshot): Map<String?, List<ChatConversation>> {
        val byView = mutableMapOf<String?, MutableList<ChatConversation>>()
        for (conversation in snapshot.conversations) {
            val viewKey = conversation.roleId // null = 管家
            val mapped = ChatConversation(
                id = conversation.id,
                title = conversation.title,
                updatedAt = parseEpochMillis(conversation.updatedAt),
                messages = conversation.messages.map { message ->
                    ChatMessage(
                        id = message.id,
                        fromButler = message.role == "assistant",
                        text = message.content,
                        senderRoleId =
                            if (conversation.roleId != null && message.role == "assistant") conversation.roleId
                            else null,
                    )
                },
            )
            byView.getOrPut(viewKey) { mutableListOf() }.add(mapped)
        }
        return byView
    }

    private fun parseEpochMillis(iso: String): Long = try {
        Instant.parse(iso).toEpochMilli()
    } catch (e: DateTimeParseException) {
        0L
    }

    // ── 活动统计（FR-38，快照全量口径）───────────────────────────────

    /**
     * scope 聚合：
     * - all：桌面全局指标（memoryCount 为真实全局数）；
     * - butler/角色：tasks/conversations 按归属聚合；**记忆数快照不可得 → null（"—"）**。
     * 时间窗 [window] 呈现快照全量口径（13.2 不做时间筛选，FR-38 交互走
     * schemaVersion 演进——评审决策 C：另立 story 扩展桌面 metrics schema 后实现）。
     */
    fun activityMetrics(snapshot: DesktopSnapshot, scopeId: String, @Suppress("UNUSED_PARAMETER") window: ActivityWindow): Map<MetricType, Int?> {
        if (scopeId == MetricScope.ALL) {
            return mapOf(
                MetricType.TASK_TOTAL to snapshot.dashboard.metrics.taskCount.toInt(),
                MetricType.MEMORY_COUNT to snapshot.dashboard.metrics.memoryCount.toInt(),
                MetricType.CONVERSATION_COUNT to snapshot.dashboard.metrics.conversationCount.toInt(),
                MetricType.PENDING_TASKS to snapshot.dashboard.metrics.pendingTaskCount.toInt(),
            )
        }
        val tasks = snapshot.tasks.filter {
            if (scopeId == MetricScope.BUTLER) it.ownerType == "butler" else it.roleId == scopeId
        }
        val conversations = snapshot.conversations.count {
            if (scopeId == MetricScope.BUTLER) it.roleId == null else it.roleId == scopeId
        }
        return mapOf(
            MetricType.TASK_TOTAL to tasks.size,
            MetricType.MEMORY_COUNT to null,
            MetricType.CONVERSATION_COUNT to conversations,
            MetricType.PENDING_TASKS to tasks.count { !it.isCompleted },
        )
    }

    // ── 时间格式化 ────────────────────────────────────────────────────

    /** ISO 8601 → 相对时间（口径同 ChatScreen formatRelativeTime）；null → 「暂无活动」；非法原样返回。 */
    fun formatRelativeTimeIso(iso: String?, nowMs: Long): String {
        if (iso == null) return "暂无活动"
        val at = try {
            Instant.parse(iso).toEpochMilli()
        } catch (e: DateTimeParseException) {
            return iso
        }
        val diff = nowMs - at
        val minutes = diff / 60_000L
        if (minutes < 1) return "刚刚"
        if (minutes < 60) return "${minutes}分钟前"
        val hours = minutes / 60
        if (hours < 24) return "${hours}小时前"
        // 跨日钟面用系统时区（评审 P11：UTC 会在东八区偏 8 小时；相对差值仍按 epoch 计算，不受影响）
        val t = Instant.ofEpochMilli(at).atZone(ZoneId.systemDefault())
        return "%d月%d日 %02d:%02d".format(Locale.ROOT, t.monthValue, t.dayOfMonth, t.hour, t.minute)
    }

    /**
     * dataCutoffAt 混合格式统一（13.1 deferred 收口）：RFC3339 → 「M月D日 HH:mm」
     * （有时分显示日期+时分，系统时区——评审 P11）；纯日期 → 「M月D日」；非法原样返回。
     */
    fun formatDataCutoff(value: String): String = try {
        val instant = Instant.parse(value)
        val t = instant.atZone(ZoneId.systemDefault())
        "%d月%d日 %02d:%02d".format(Locale.ROOT, t.monthValue, t.dayOfMonth, t.hour, t.minute)
    } catch (e: DateTimeParseException) {
        try {
            val date = LocalDate.parse(value)
            "%d月%d日".format(Locale.ROOT, date.monthValue, date.dayOfMonth)
        } catch (e2: DateTimeParseException) {
            value
        }
    }

    /** "YYYY-MM-DD" → 「M月D日 · 周X」；非法原样返回。 */
    private fun formatDateLabel(date: String): String = try {
        val d = LocalDate.parse(date)
        val week = "周" + "一二三四五六日"[d.dayOfWeek.value - 1]
        "%d月%d日 · %s".format(Locale.ROOT, d.monthValue, d.dayOfMonth, week)
    } catch (e: DateTimeParseException) {
        date
    }

    /** weekStart/weekEnd → 「第 N 周 · M月D日 — M月D日」；非法原样拼接。 */
    private fun formatWeekLabel(weekStart: String, weekEnd: String): String = try {
        val start = LocalDate.parse(weekStart)
        val end = LocalDate.parse(weekEnd)
        val week = start.get(WeekFields.ISO.weekOfWeekBasedYear())
        "第 %d 周 · %d月%d日 — %d月%d日".format(
            Locale.ROOT, week, start.monthValue, start.dayOfMonth, end.monthValue, end.dayOfMonth,
        )
    } catch (e: DateTimeParseException) {
        "$weekStart — $weekEnd"
    }
}
