package com.egosync.companion.sync

import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject

/**
 * Story 13.2：桌面快照领域模型（严格对齐桌面 `egosync-app/src-tauri/src/models/snapshot.rs`，
 * 字段名 camelCase 一一对应）。解析入口 [SnapshotParser]。
 *
 * 负向契约（桌面负向断言的 Android 镜像）：**记忆条目内容永不在快照中**——
 * `memoryCount`/`newMemoriesCount` 仅作为数字指标出现；`personality_prompt`/
 * `skills_config` 不上机。SnapshotParserTest 以反射扫描锁定。
 */

/** 快照业务结构版本（独立于协议层 protocolVersion，未知版本显式失败）。 */
const val SNAPSHOT_SCHEMA_VERSION = 1

/** 快照结构损坏（JSON 坏/缺字段/版本未知）——显式失败，不静默降级。 */
class SnapshotFormatException(message: String, cause: Throwable? = null) : Exception(message, cause)

data class DesktopSnapshot(
    val schemaVersion: Int,
    /** 快照生成时刻（ISO 8601 UTC）。 */
    val generatedAt: String,
    /** 数据截止时间：截断后被截域保留数据的最旧时间戳；未截断为 null。 */
    val dataCutoffAt: String?,
    val truncated: Boolean,
    /** 被截断的域名清单（conversations/briefings/weeklyReviews）。 */
    val truncatedDomains: List<String>,
    val roles: List<SnapshotRole>,
    val tasks: List<SnapshotTask>,
    val dashboard: SnapshotDashboard,
    val conversations: List<SnapshotConversation>,
    val briefings: List<SnapshotBriefing>,
    val weeklyReviews: List<SnapshotWeeklyReview>,
    val notifications: List<SnapshotNotification>,
)

data class SnapshotRole(
    val id: String,
    val name: String,
    val icon: String,
    val color: String,
    val goal: String,
    val status: String,
    val energy: Int,
    val proactivityLevel: String,
)

data class SnapshotTask(
    val id: String,
    val ownerType: String,
    val roleId: String?,
    val title: String,
    val deadline: String?,
    val quadrant: String,
    val isBigRock: Boolean,
    val isCompleted: Boolean,
    val protectionStatus: String,
    val roleName: String?,
    val roleColor: String?,
)

data class SnapshotDashboard(
    val statuses: List<DashboardStatus>,
    val metrics: SnapshotMetrics,
)

data class DashboardStatus(
    val roleId: String,
    val roleName: String,
    val roleIcon: String,
    val roleColor: String,
    val energy: Int,
    val pendingTasksCount: Long,
    val lastActiveAt: String?,
    val hasUrgent: Boolean,
)

data class SnapshotMetrics(
    val taskCount: Long,
    val memoryCount: Long,
    val conversationCount: Long,
    val pendingTaskCount: Long,
    val generatedAt: String,
)

data class SnapshotConversation(
    val id: String,
    val roleId: String?,
    val title: String,
    val updatedAt: String,
    val messages: List<SnapshotMessage>,
)

data class SnapshotMessage(
    val id: String,
    /** 发言角色："user" / "assistant"。 */
    val role: String,
    val content: String,
    val thinkingContent: String,
    val isComplete: Boolean,
    val createdAt: String,
)

data class SnapshotBriefing(
    val id: String,
    val content: String,
    val date: String,
)

data class SnapshotWeeklyReview(
    val id: String,
    val weekStart: String,
    val weekEnd: String,
    val summary: String,
    /** JSON 字符串：`{"<roleId>": {"energy": Int, "energyUpdatedAt": String?}}`（各角色当前能量快照，非时间序列）。 */
    val energyTrends: String,
    /** JSON 字符串：`[{"id","title","isCompleted","completedAt","roleName"}]`。 */
    val bigrockStatus: String,
    val newMemoriesCount: Long,
)

data class SnapshotNotification(
    val id: String,
    val roleId: String,
    val level: String,
    val content: String,
    val createdAt: String,
    val roleName: String,
    val roleIcon: String,
    val roleColor: String,
)

/**
 * 快照 JSON → [DesktopSnapshot]。schemaVersion 未知、必填字段缺失、JSON 损坏
 * 均显式抛 [SnapshotFormatException]（消费方校验职责，13.1 裁决）。
 */
object SnapshotParser {

    fun parse(json: String): DesktopSnapshot = try {
        val root = JSONObject(json)
        val schemaVersion = root.getInt("schemaVersion")
        if (schemaVersion != SNAPSHOT_SCHEMA_VERSION) {
            throw SnapshotFormatException("未知快照 schemaVersion=$schemaVersion，本端支持 $SNAPSHOT_SCHEMA_VERSION")
        }
        DesktopSnapshot(
            schemaVersion = schemaVersion,
            generatedAt = root.getString("generatedAt"),
            dataCutoffAt = root.optStringOrNull("dataCutoffAt"),
            truncated = root.getBoolean("truncated"),
            truncatedDomains = root.getJSONArray("truncatedDomains").toStrings(),
            roles = root.getJSONArray("roles").map { it.toRole() },
            tasks = root.getJSONArray("tasks").map { it.toTask() },
            dashboard = root.getJSONObject("dashboard").toDashboard(),
            conversations = root.getJSONArray("conversations").map { it.toConversation() },
            briefings = root.getJSONArray("briefings").map { it.toBriefing() },
            weeklyReviews = root.getJSONArray("weeklyReviews").map { it.toWeeklyReview() },
            notifications = root.getJSONArray("notifications").map { it.toNotification() },
        )
    } catch (e: SnapshotFormatException) {
        throw e
    } catch (e: JSONException) {
        throw SnapshotFormatException("快照 JSON 结构无效: ${e.message}", e)
    }

    // ── 各域解析 ──────────────────────────────────────────────────────

    private fun JSONObject.toRole() = SnapshotRole(
        id = getString("id"),
        name = getString("name"),
        icon = getString("icon"),
        color = getString("color"),
        goal = getString("goal"),
        status = getString("status"),
        energy = getInt("energy"),
        proactivityLevel = getString("proactivityLevel"),
    )

    private fun JSONObject.toTask() = SnapshotTask(
        id = getString("id"),
        ownerType = getString("ownerType"),
        roleId = optStringOrNull("roleId"),
        title = getString("title"),
        deadline = optStringOrNull("deadline"),
        quadrant = getString("quadrant"),
        isBigRock = getBoolean("isBigRock"),
        isCompleted = getBoolean("isCompleted"),
        protectionStatus = getString("protectionStatus"),
        roleName = optStringOrNull("roleName"),
        roleColor = optStringOrNull("roleColor"),
    )

    private fun JSONObject.toDashboard() = SnapshotDashboard(
        statuses = getJSONArray("statuses").map { json ->
            DashboardStatus(
                roleId = json.getString("roleId"),
                roleName = json.getString("roleName"),
                roleIcon = json.getString("roleIcon"),
                roleColor = json.getString("roleColor"),
                energy = json.getInt("energy"),
                pendingTasksCount = json.getLong("pendingTasksCount"),
                lastActiveAt = json.optStringOrNull("lastActiveAt"),
                hasUrgent = json.getBoolean("hasUrgent"),
            )
        },
        metrics = SnapshotMetrics(
            taskCount = getJSONObject("metrics").getLong("taskCount"),
            memoryCount = getJSONObject("metrics").getLong("memoryCount"),
            conversationCount = getJSONObject("metrics").getLong("conversationCount"),
            pendingTaskCount = getJSONObject("metrics").getLong("pendingTaskCount"),
            generatedAt = getJSONObject("metrics").getString("generatedAt"),
        ),
    )

    private fun JSONObject.toConversation() = SnapshotConversation(
        id = getString("id"),
        roleId = optStringOrNull("roleId"),
        title = getString("title"),
        updatedAt = getString("updatedAt"),
        messages = getJSONArray("messages").map { m ->
            SnapshotMessage(
                id = m.getString("id"),
                role = m.getString("role"),
                content = m.getString("content"),
                thinkingContent = m.getString("thinkingContent"),
                isComplete = m.getBoolean("isComplete"),
                createdAt = m.getString("createdAt"),
            )
        },
    )

    private fun JSONObject.toBriefing() = SnapshotBriefing(
        id = getString("id"),
        content = getString("content"),
        date = getString("date"),
    )

    private fun JSONObject.toWeeklyReview() = SnapshotWeeklyReview(
        id = getString("id"),
        weekStart = getString("weekStart"),
        weekEnd = getString("weekEnd"),
        summary = getString("summary"),
        energyTrends = getString("energyTrends"),
        bigrockStatus = getString("bigrockStatus"),
        newMemoriesCount = getLong("newMemoriesCount"),
    )

    private fun JSONObject.toNotification() = SnapshotNotification(
        id = getString("id"),
        roleId = getString("roleId"),
        level = getString("level"),
        content = getString("content"),
        createdAt = getString("createdAt"),
        roleName = getString("roleName"),
        roleIcon = getString("roleIcon"),
        roleColor = getString("roleColor"),
    )

    // ── org.json 工具 ────────────────────────────────────────────────

    private fun JSONObject.optStringOrNull(key: String): String? =
        if (isNull(key)) null else optString(key, null)

    private fun JSONArray.toStrings(): List<String> =
        List(length()) { i -> getString(i) }

    private fun <T> JSONArray.map(transform: (JSONObject) -> T): List<T> =
        List(length()) { i -> transform(getJSONObject(i)) }
}
