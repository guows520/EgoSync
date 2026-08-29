package com.egosync.companion.sync

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 快照领域模型解析契约（镜像桌面 models/snapshot.rs schema，schemaVersion=1）：
 * 字段名 camelCase 严格对齐、未知版本显式失败（13.2 消费方校验职责）、
 * 记忆内容永不进快照（桌面负向断言的 Android 镜像防呆）。
 */
class SnapshotParserTest {

    /** 最小合法快照 JSON（七域俱全，字段名对齐桌面 serde camelCase 产物）。 */
    private fun snapshotJson(
        schemaVersion: Int = 1,
        rolesJson: String = """[{"id":"role-pm","name":"产品经理","icon":"target","color":"#6366F1","goal":"跟进评审","status":"active","energy":82,"proactivityLevel":"moderate"}]""",
        tasksJson: String = """[{"id":"t-1","ownerType":"role","roleId":"role-pm","title":"评审材料","deadline":"周五","quadrant":"Q2","isBigRock":true,"isCompleted":false,"protectionStatus":"normal","roleName":"产品经理","roleColor":"#6366F1"}]""",
        conversationsJson: String = """[{"id":"c-1","roleId":"role-pm","title":"评审","updatedAt":"2026-08-29T08:00:00Z","messages":[{"id":"m-1","role":"assistant","content":"已备好","thinkingContent":"","isComplete":true,"createdAt":"2026-08-29T08:00:00Z"}]}]""",
        briefingsJson: String = """[{"id":"b-1","content":"今天最重要的事是评审","date":"2026-08-29"}]""",
        notificationsJson: String = """[{"id":"n-1","roleId":"role-pm","level":"tap","content":"提醒","createdAt":"2026-08-29T10:30:00Z","roleName":"产品经理","roleIcon":"target","roleColor":"#6366F1"}]""",
    ): String = """
        {"schemaVersion":$schemaVersion,"generatedAt":"2026-08-29T12:00:00Z","dataCutoffAt":null,
         "truncated":false,"truncatedDomains":[],
         "roles":$rolesJson,"tasks":$tasksJson,
         "dashboard":{"statuses":[{"roleId":"role-pm","roleName":"产品经理","roleIcon":"target",
                       "roleColor":"#6366F1","energy":82,"pendingTasksCount":3,
                       "lastActiveAt":"2026-08-29T11:00:00Z","hasUrgent":false}],
                      "metrics":{"taskCount":14,"memoryCount":38,"conversationCount":26,
                                 "pendingTaskCount":3,"generatedAt":"2026-08-29T12:00:00Z"}},
         "conversations":$conversationsJson,
         "briefings":$briefingsJson,
         "weeklyReviews":[{"id":"r-1","weekStart":"2026-08-24","weekEnd":"2026-08-30",
                           "summary":"稳步推进","energyTrends":"{\"role-pm\":{\"energy\":82,\"energyUpdatedAt\":\"2026-08-29T10:00:00Z\"}}",
                           "bigrockStatus":"[{\"id\":\"br-1\",\"title\":\"评审材料\",\"isCompleted\":false,\"completedAt\":null,\"roleName\":\"产品经理\"}]",
                           "newMemoriesCount":2}],
         "notifications":$notificationsJson}
    """.trimIndent()

    @Test
    fun `完整快照解析出七域与元数据`() {
        val snapshot = SnapshotParser.parse(snapshotJson())

        assertEquals(1, snapshot.schemaVersion)
        assertEquals("2026-08-29T12:00:00Z", snapshot.generatedAt)
        assertNull(snapshot.dataCutoffAt)
        assertEquals(false, snapshot.truncated)
        assertEquals("role-pm", snapshot.roles.single().id)
        assertEquals(82, snapshot.roles.single().energy)
        assertEquals("Q2", snapshot.tasks.single().quadrant)
        assertEquals(3, snapshot.dashboard.statuses.single().pendingTasksCount)
        assertEquals(14L, snapshot.dashboard.metrics.taskCount)
        assertEquals("已备好", snapshot.conversations.single().messages.single().content)
        assertEquals("2026-08-29", snapshot.briefings.single().date)
        assertEquals("2026-08-24", snapshot.weeklyReviews.single().weekStart)
        assertEquals("tap", snapshot.notifications.single().level)
    }

    @Test
    fun `截断元数据字段完整解析`() {
        // WHY：AC4 依赖 truncated/truncatedDomains/dataCutoffAt 明示截断——
        // 任一字段解析丢失都会让手机拿残缺数据冒充完整
        val json = snapshotJson()
            .replace("\"dataCutoffAt\":null", "\"dataCutoffAt\":\"2026-08-01\"")
            .replace("\"truncated\":false", "\"truncated\":true")
            .replace("\"truncatedDomains\":[]", "\"truncatedDomains\":[\"conversations\",\"briefings\"]")

        val snapshot = SnapshotParser.parse(json)

        assertEquals(true, snapshot.truncated)
        assertEquals("2026-08-01", snapshot.dataCutoffAt)
        assertEquals(listOf("conversations", "briefings"), snapshot.truncatedDomains)
    }

    @Test
    fun `未知 schemaVersion 显式失败不静默`() {
        // WHY：schemaVersion 是快照结构演进通道（≠ protocolVersion）——未知版本
        // 静默解析会按旧结构错位读字段，渲染出张冠李戴的数据
        assertThrows(SnapshotFormatException::class.java) {
            SnapshotParser.parse(snapshotJson(schemaVersion = 2))
        }
    }

    @Test
    fun `损坏 JSON 显式失败`() {
        assertThrows(SnapshotFormatException::class.java) {
            SnapshotParser.parse("{broken")
        }
    }

    @Test
    fun `缺失必填字段显式失败`() {
        // WHY：缺字段若被默认值静默吞掉，手机会渲染空域冒充「桌面没有数据」
        assertThrows(SnapshotFormatException::class.java) {
            SnapshotParser.parse("""{"schemaVersion":1}""")
        }
    }

    @Test
    fun `空域与可空字段合法`() {
        // WHY：真实快照可能为空域（无任务/无通知）——空数组与 null 是合法形态，
        // 解析层不得崩溃（13.1 教训 #4：空域守卫）
        val json = snapshotJson(
            rolesJson = "[]", tasksJson = "[]", conversationsJson = "[]",
            briefingsJson = "[]", notificationsJson = "[]",
        ).replace("\"lastActiveAt\":\"2026-08-29T11:00:00Z\"", "\"lastActiveAt\":null")

        val snapshot = SnapshotParser.parse(json)

        assertTrue(snapshot.roles.isEmpty())
        assertTrue(snapshot.tasks.isEmpty())
        assertTrue(snapshot.conversations.isEmpty())
        assertTrue(snapshot.briefings.isEmpty())
        assertTrue(snapshot.notifications.isEmpty())
        // lastActiveAt 为 null 的用例真正覆盖 Option 字段缺失/空容忍
        //（桌面 serde Option 序列化规则；13.2 评审 P12：此前断言非 null 未测 null 分支）
        assertNull(snapshot.dashboard.statuses.single().lastActiveAt)
    }

    @Test
    fun `快照模型不存在记忆条目内容字段`() {
        // WHY：记忆内容永不进快照（桌面 snapshot_scope_completeness_and_memory_exclusion
        // 负向断言的镜像）——模型层出现 memories/memorySources 类字段即契约击穿；
        // personality_prompt/skills_config 同属 §2 负向契约不上机字段（评审 P15）
        // Java 反射扫描（无 kotlin-reflect 依赖）。
        val modelClasses = listOf(
            DesktopSnapshot::class.java, SnapshotRole::class.java, SnapshotTask::class.java,
            SnapshotDashboard::class.java, DashboardStatus::class.java, SnapshotMetrics::class.java,
            SnapshotConversation::class.java, SnapshotMessage::class.java,
            SnapshotBriefing::class.java, SnapshotWeeklyReview::class.java,
            SnapshotNotification::class.java,
        )
        val offenders = modelClasses
            .flatMap { it.declaredFields.toList() }
            .filter { field ->
                val name = field.name.lowercase()
                (name.contains("memor") && name != "memorycount" && name != "newmemoriescount") ||
                    name.contains("personality") || name.contains("skillsconfig")
            }
        assertTrue("快照模型不得携带记忆内容/personality/skills 字段: $offenders", offenders.isEmpty())
    }
}
