package com.egosync.companion.ui.chat

import com.egosync.companion.sync.ChatMessage
import com.egosync.companion.sync.DesktopSnapshot
import com.egosync.companion.sync.SnapshotDashboard
import com.egosync.companion.sync.SnapshotConversation
import com.egosync.companion.sync.SnapshotMessage
import com.egosync.companion.sync.DashboardStatus
import com.egosync.companion.sync.SnapshotMetrics
import com.egosync.companion.sync.SnapshotRole
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 多会话状态机意图验证（规格 FR-3 / FR-6，I/O 契约六场景的 VM 侧语义）。
 *
 * 13.2 换装后：会话列表由快照 conversations 域派生（种子会话固定在测试夹具快照中），
 * 回复轮换/提案等生成性内容见 [ChatDemoData]。VM 内所有 delay（思考/工具阶段/打字机）
 * 由测试调度器推进，可用 advanceTimeBy 精确停在「流式进行中」验证中断守卫。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ChatViewModelTest {

    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    // 测试夹具种子（镜像旧 mock 种子会话，内容稳定即可，无业务断言依赖其文案）
    private val butlerGreeting = "早上好 boss。今天最重要的一件事是下午 2 点的产品评审。"
    private val butlerHistoryTitle = "上季度 OKR 整理"
    private val butlerCurrentTitle = "今日概览"
    private val butlerHistoryMsgs = listOf(
        ChatMessage("h-1", fromButler = false, text = "帮我把上季度 OKR 整理一下。"),
        ChatMessage("h-2", fromButler = true, text = "好的，我按三个 O 分别列了完成度。"),
    )
    private val rolePmSeedMsgs = listOf(
        ChatMessage("r-pm-1", fromButler = true, senderRoleId = "role-pm", text = "boss，评审材料已备好。"),
    )

    /** 夹具快照：管家 2 会话（当前/历史）+ role-pm 1 会话 + 3 角色。 */
    private fun fixtureSnapshot(): DesktopSnapshot = DesktopSnapshot(
        schemaVersion = 1,
        generatedAt = "2026-08-25T08:00:00Z",
        dataCutoffAt = null,
        truncated = false,
        truncatedDomains = emptyList(),
        roles = listOf(
            SnapshotRole("role-pm", "产品经理", "target", "#4F46E5", "", "", 82, "moderate"),
            SnapshotRole("role-father", "父亲", "home", "#16A34A", "", "", 56, "moderate"),
            SnapshotRole("role-learner", "学习者", "book-open", "#2563EB", "", "", 33, "moderate"),
        ),
        tasks = emptyList(),
        dashboard = SnapshotDashboard(
            statuses = listOf(
                DashboardStatus("role-pm", "产品经理", "target", "#4F46E5", 82, 3,
                    "2026-08-25T07:50:00Z", false),
                DashboardStatus("role-father", "父亲", "home", "#16A34A", 56, 1,
                    "2026-08-24T20:00:00Z", false),
                DashboardStatus("role-learner", "学习者", "book-open", "#2563EB", 33, 0,
                    "2026-08-21T22:00:00Z", false),
            ),
            metrics = SnapshotMetrics(0, 0, 0, 0, "2026-08-25T08:00:00Z"),
        ),
        conversations = listOf(
            SnapshotConversation(
                id = "conv-butler-current", roleId = null, title = butlerCurrentTitle,
                updatedAt = "2026-08-25T06:00:00Z",
                messages = listOf(
                    SnapshotMessage("m-1", "assistant", butlerGreeting, "", true, "2026-08-25T06:00:00Z"),
                ),
            ),
            SnapshotConversation(
                id = "conv-butler-history", roleId = null, title = butlerHistoryTitle,
                updatedAt = "2026-08-24T06:00:00Z",
                messages = butlerHistoryMsgs.mapIndexed { i, m ->
                    SnapshotMessage(m.id, if (m.fromButler) "assistant" else "user", m.text, "", true,
                        "2026-08-24T0${6 + i}:00:00Z")
                },
            ),
            SnapshotConversation(
                id = "conv-pm", roleId = "role-pm", title = "评审材料准备",
                updatedAt = "2026-08-25T07:00:00Z",
                messages = rolePmSeedMsgs.map { m ->
                    SnapshotMessage(m.id, "assistant", m.text, "", true, "2026-08-25T07:00:00Z")
                },
            ),
        ),
        briefings = emptyList(),
        weeklyReviews = emptyList(),
        notifications = emptyList(),
    )

    private fun newVm(
        store: SnapshotStore = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) },
    ): ChatViewModel = ChatViewModel(store)

    /** 初始种子会话中「今日概览」（当前会话，最新在前）。 */
    private fun ChatUiState.currentConversation() =
        conversations.first { it.id == currentConversationId }

    // ── 场景 1：新建会话归属与角色隔离 ─────────────────────────────────

    @Test
    fun `新建会话归属当前角色且不串到其他角色`() {
        // WHY：桌面端每个角色独立维护会话列表；若新建会话串到别的角色，
        // 用户切到角色视图会看到不属于自己的对话，属于数据越权泄漏。
        val vm = newVm()

        vm.newConversation()
        val newId = vm.uiState.value.currentConversationId
        assertEquals(3, vm.uiState.value.conversations.size) // 2 种子 + 1 新建
        assertTrue(vm.uiState.value.messages.isEmpty()) // 新会话为空消息流

        // 切到角色视图：新会话不得出现
        vm.selectRole("role-pm")
        assertFalse(vm.uiState.value.conversations.any { it.id == newId })
        assertEquals(1, vm.uiState.value.conversations.size) // 仅 pm 种子

        // 切回管家：新会话仍在（归属未丢）
        vm.selectRole(null)
        assertTrue(vm.uiState.value.conversations.any { it.id == newId })
    }

    @Test
    fun `新建会话设为当前且列表最新在前`() {
        // WHY：新建后必须立即激活，否则用户发的消息会写进旧会话，
        // 「开新话题」的心智模型直接破裂。
        val vm = newVm()
        val before = vm.uiState.value.currentConversationId

        vm.newConversation()

        assertNotEquals(before, vm.uiState.value.currentConversationId)
        assertEquals(
            vm.uiState.value.currentConversationId,
            vm.uiState.value.conversations.first().id,
        )
    }

    // ── 场景 2：切换会话换消息流（含卡片态规则）────────────────────────

    @Test
    fun `切换会话换消息流且可切回`() {
        // WHY：历史下拉的唯一价值就是「换到哪个会话就看到哪个会话的完整消息流」；
        // 切换后消息流不随动 = 功能失效。
        val vm = newVm()
        val historyConv = vm.uiState.value.conversations.first { it.title == butlerHistoryTitle }
        assertEquals(listOf(butlerGreeting), vm.uiState.value.messages.map { it.text })

        vm.selectConversation(historyConv.id)
        assertEquals(butlerHistoryMsgs.map { it.text }, vm.uiState.value.messages.map { it.text })
        assertEquals(historyConv.id, vm.uiState.value.currentConversationId)

        // 切回当前种子会话，消息流恢复
        val currentConv = vm.uiState.value.conversations.first { it.title == butlerCurrentTitle }
        vm.selectConversation(currentConv.id)
        assertEquals(listOf(butlerGreeting), vm.uiState.value.messages.map { it.text })
    }

    @Test
    fun `角色视图切换会话时卡片态清空`() {
        // WHY：拆分提案/建议卡/角色涌现卡属管家对话流；若切到角色会话仍显示，
        // 用户会在错误的上下文里点「确认」，把管家决策误挂给角色（镜像桌面按 conversation 隔离）。
        val vm = newVm()
        vm.selectRole("role-pm")
        val seedId = vm.uiState.value.conversations.first().id
        vm.newConversation() // pm 现在有 2 个会话：空会话 + 种子

        vm.selectConversation(seedId)

        assertEquals(rolePmSeedMsgs.map { it.text }, vm.uiState.value.messages.map { it.text })
        assertTrue(vm.uiState.value.actionCards.isEmpty())
        assertEquals(null, vm.uiState.value.decomposition)
        assertEquals(null, vm.uiState.value.roleProposal)
    }

    @Test
    fun `管家视图切换会话后卡片态同样清空`() {
        // WHY：卡片属触发它的那段对话（按 conversation 隔离）；若切到别的管家会话
        // 仍显示旧会话的提案卡，用户会在不相干的会话里点「确认」，决策挂错上下文。
        val vm = newVm()
        // 管家视图走两轮完整回复：第 2 轮后浮现建议卡/拆分提案/角色涌现卡
        vm.sendMessage("第一轮")
        dispatcher.scheduler.advanceUntilIdle()
        vm.sendMessage("第二轮")
        dispatcher.scheduler.advanceUntilIdle()
        assertFalse(vm.uiState.value.actionCards.isEmpty())

        val okrId = vm.uiState.value.conversations.first { it.title == butlerHistoryTitle }.id
        vm.selectConversation(okrId)

        assertTrue(vm.uiState.value.actionCards.isEmpty())
        assertEquals(null, vm.uiState.value.decomposition)
        assertEquals(null, vm.uiState.value.roleProposal)
    }

    @Test
    fun `切走再切回复原视图原会话`() {
        // WHY：往返切换（管家→角色→管家）若每次都重置为「最新会话」，
        // 用户正在看的会话会被静默换掉——上下文丢失且无任何提示。
        val vm = newVm()
        val okrId = vm.uiState.value.conversations.first { it.title == butlerHistoryTitle }.id
        vm.selectConversation(okrId)

        vm.selectRole("role-pm")
        vm.selectRole(null)

        assertEquals(okrId, vm.uiState.value.currentConversationId)
        assertEquals(butlerHistoryMsgs.map { it.text }, vm.uiState.value.messages.map { it.text })
    }

    @Test
    fun `切换到未种子化角色自动建会话且消息可持久`() {
        // WHY：若无会话兜底，currentConversationId 悬空为空串，persist 静默跳过——
        // 用户看到完整对话，切走切回后消息全部消失（界面与数据自相矛盾）。
        val vm = newVm()

        vm.selectRole("role-unknown")
        assertEquals(1, vm.uiState.value.conversations.size)

        vm.sendMessage("测试消息")
        dispatcher.scheduler.advanceUntilIdle()

        vm.selectRole(null)
        vm.selectRole("role-unknown")
        assertTrue(vm.uiState.value.messages.any { it.text == "测试消息" })
    }

    // ── 场景 3：删除当前会话回退 / 全删兜底 ────────────────────────────

    @Test
    fun `删除当前会话回退到剩余最新会话`() {
        // WHY：删的正是正在看的会话时必须有兜底落点，否则 currentConversationId
        // 悬空，后续发消息会写进不存在的会话（persist 静默丢失）。
        val vm = newVm()
        val okrConv = vm.uiState.value.conversations.first { it.title == butlerHistoryTitle }

        vm.deleteConversation(vm.uiState.value.currentConversationId)

        assertEquals(okrConv.id, vm.uiState.value.currentConversationId)
        assertEquals(butlerHistoryMsgs.map { it.text }, vm.uiState.value.messages.map { it.text })
        assertEquals(1, vm.uiState.value.conversations.size)
    }

    @Test
    fun `删光所有会话自动新建空会话兜底`() {
        // WHY：会话列表为空时界面不能进入「无当前会话」死态——
        // 输入框永远无处可发，用户只能重启 App，这是体验断裂。
        val vm = newVm()
        vm.deleteConversation(vm.uiState.value.currentConversationId) // 剩 OKR 种子
        vm.deleteConversation(vm.uiState.value.currentConversationId) // 全删

        assertEquals(1, vm.uiState.value.conversations.size)
        assertTrue(vm.uiState.value.currentConversationId.isNotEmpty())
        assertTrue(vm.uiState.value.messages.isEmpty())
        assertTrue(vm.uiState.value.conversations.first().title.isEmpty())
    }

    // ── 场景 4：删除其他会话不影响当前 ─────────────────────────────────

    @Test
    fun `删除其他会话仅移除该会话当前不变`() {
        // WHY：删除操作必须是精确的——误删当前上下文会让用户丢失正在进行的对话。
        val vm = newVm()
        val currentId = vm.uiState.value.currentConversationId
        val okrId = vm.uiState.value.conversations.first { it.title == butlerHistoryTitle }.id

        vm.deleteConversation(okrId)

        assertEquals(currentId, vm.uiState.value.currentConversationId)
        assertEquals(listOf(butlerGreeting), vm.uiState.value.messages.map { it.text })
        assertEquals(1, vm.uiState.value.conversations.size)
    }

    // ── 场景 5：空会话首条消息生成标题 ─────────────────────────────────

    @Test
    fun `空会话首条消息生成标题且超长截断加省略号`() {
        // WHY：历史下拉靠标题辨认会话；空标题会话永远显示「新对话」，
        // 多条之后用户将无法找回任何一段对话（镜像桌面会话命名规则）。
        val vm = newVm()
        vm.newConversation()

        vm.sendMessage("你好") // 同步落标题，无需推进调度器
        assertEquals("你好", vm.uiState.value.currentConversation().title)

        vm.newConversation()
        vm.sendMessage("abcdefghijklmnopqrst") // 20 字符 > 16
        assertEquals("abcdefghijklmnop…", vm.uiState.value.currentConversation().title)
    }

    // ── 场景 6：流式中断守卫（新建 / 切换后 responding=false）──────────

    @Test
    fun `流式进行中新建会话立即中断且不回写新会话`() {
        // WHY：打字机协程若不被取消，后续字符会继续写进「当前会话」——
        // 而此时当前会话已换成新的空会话，旧回复污染新对话且 stop 按钮失效。
        val vm = newVm()
        vm.sendMessage("帮我看看明天的安排")
        dispatcher.scheduler.advanceTimeBy(1800) // 思考 700ms + 工具阶段进行中，打字机已启动
        assertTrue(vm.uiState.value.responding)

        vm.newConversation()

        assertFalse(vm.uiState.value.responding)
        assertFalse(vm.uiState.value.thinking)
        assertEquals(emptyList<ChatMessage>(), vm.uiState.value.messages)

        // 让残留协程有机会跑完：若取消失效，这里会混入流式文本
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(emptyList<ChatMessage>(), vm.uiState.value.messages)
        assertFalse(vm.uiState.value.responding)
    }

    @Test
    fun `流式进行中切换会话立即中断且不泄漏到目标会话`() {
        // WHY：同上的跨会话污染路径——切到历史会话后，旧流式的剩余字符
        // 若继续写入，会让一段不相干的回复凭空出现在历史对话里。
        val vm = newVm()
        val okrId = vm.uiState.value.conversations.first { it.title == butlerHistoryTitle }.id

        vm.sendMessage("帮我看看明天的安排")
        dispatcher.scheduler.advanceTimeBy(1800)
        assertTrue(vm.uiState.value.responding)

        vm.selectConversation(okrId)

        assertFalse(vm.uiState.value.responding)
        assertEquals(butlerHistoryMsgs.map { it.text }, vm.uiState.value.messages.map { it.text })

        dispatcher.scheduler.advanceUntilIdle()
        // 目标会话保持原样：种子消息一条不多一条不少
        assertEquals(butlerHistoryMsgs.map { it.text }, vm.uiState.value.messages.map { it.text })
    }

    // ── 场景 4：快照换装语义（AC2 核心分支，评审 P14）──────────────────

    @Test
    fun `同对象首发不重建_本地暂存不被首帧抹掉`() {
        // WHY：构造时已加载的快照会作为 collect 首帧再次投递；无同一性守卫时
        // 首帧重建会 seedFromStore 抹掉首帧之前用户已完成的本地暂存（新建会话）。
        val vm = newVm()

        vm.newConversation()
        dispatcher.scheduler.advanceUntilIdle() // 驱动 collect 首帧

        assertEquals(3, vm.uiState.value.conversations.size) // 2 种子 + 1 本地新建
    }

    @Test
    fun `二次applySnapshot后消息与会话随新快照刷新`() {
        // WHY：AC2 的核心承诺——STATE_DELTA 全量替换后，消息流与会话列表必须随
        // 新快照刷新（含当前会话内桌面侧新增的消息），否则换装名存实亡；
        // 同时当前会话保持不跳（快照替换不重置用户视图）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = newVm(store)
        dispatcher.scheduler.advanceUntilIdle()

        val second = fixtureSnapshot().copy(
            conversations = fixtureSnapshot().conversations.map { conv ->
                if (conv.id == "conv-butler-current") {
                    conv.copy(
                        messages = conv.messages + SnapshotMessage(
                            "m-2", "assistant", "桌面侧新增的回复", "", true, "2026-08-25T09:00:00Z",
                        ),
                    )
                } else {
                    conv
                }
            } + SnapshotConversation(
                id = "conv-butler-new", roleId = null, title = "桌面新建会话",
                updatedAt = "2026-08-25T09:30:00Z", messages = emptyList(),
            ),
        )
        store.applySnapshot(second)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(
            listOf(butlerGreeting, "桌面侧新增的回复"),
            vm.uiState.value.messages.map { it.text },
        )
        assertEquals("conv-butler-current", vm.uiState.value.currentConversationId)
        // 管家视角：2 个种子会话 + 桌面新建会话（role-pm 会话在角色视图，不计入）
        assertEquals(3, vm.uiState.value.conversations.size)
    }

    @Test
    fun `冷启动空快照兜底建会话_快照到达后重建`() {
        // WHY：离线冷启动（无缓存）时 Chat 不能进入「无当前会话」死态；
        // 快照随后到达时必须自动重建，会话列表与消息流切换到快照口径。
        val store = SnapshotStore()
        val vm = newVm(store)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(1, vm.uiState.value.conversations.size) // 兜底空会话
        assertTrue(vm.uiState.value.messages.isEmpty())

        store.applySnapshot(fixtureSnapshot())
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(listOf(butlerGreeting), vm.uiState.value.messages.map { it.text })
        assertEquals(3, vm.uiState.value.roles.size)
    }

    @Test
    fun `快照删除当前会话回落时卡片态清空`() {
        // WHY：回落换会话必须遵守 selectConversation 的同一不变量——卡片态属
        // 触发它的那段对话；桌面删除当前会话导致的回落若不清卡片，用户会在
        // 不相干的会话里确认旧提案（评审 P4）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = newVm(store)
        vm.sendMessage("第一轮")
        dispatcher.scheduler.advanceUntilIdle()
        vm.sendMessage("第二轮")
        dispatcher.scheduler.advanceUntilIdle()
        assertFalse(vm.uiState.value.actionCards.isEmpty())

        val reduced = fixtureSnapshot().copy(
            conversations = fixtureSnapshot().conversations.filterNot { it.id == "conv-butler-current" },
        )
        store.applySnapshot(reduced)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(butlerHistoryMsgs.map { it.text }, vm.uiState.value.messages.map { it.text })
        assertTrue(vm.uiState.value.actionCards.isEmpty())
        assertNull(vm.uiState.value.decomposition)
        assertTrue(vm.uiState.value.traceByMessageId.isEmpty())
    }

    @Test
    fun `当前查看角色被快照删除后回退管家视图`() {
        // WHY：角色被桌面删除后，切换器已无该角色入口；若手机仍停留在已删
        // 角色的空视图，用户没有直观路径回到管家（幽灵视图，评审 P5）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = newVm(store)
        // 先启动 collect 协程（同对象首帧跳过）：lastSnapshot 捕获发生在协程体首跑时，
        // 若先换快照再驱动调度器，守卫基线会直接捕获新快照而跳过 rebuild——生产中
        // collect 在主线程同轮启动，早于连接层帧到达，此处用 runCurrent 对齐该时序
        dispatcher.scheduler.runCurrent()
        vm.selectRole("role-pm")

        // 注意：store.roles 门面从 dashboard.statuses 派生（SnapshotMapper.toRoleCards），
        // 模拟删除角色须同步删 dashboard 中的状态行，仅删 roles 域不生效
        val reduced = fixtureSnapshot().copy(
            roles = fixtureSnapshot().roles.filterNot { it.id == "role-pm" },
            dashboard = fixtureSnapshot().dashboard.copy(
                statuses = fixtureSnapshot().dashboard.statuses.filterNot { it.roleId == "role-pm" },
            ),
            conversations = fixtureSnapshot().conversations.filterNot { it.roleId == "role-pm" },
        )
        store.applySnapshot(reduced)
        dispatcher.scheduler.advanceUntilIdle()

        assertNull(vm.uiState.value.activeRoleId)
        assertTrue(vm.uiState.value.roles.none { it.id == "role-pm" })
    }

    @Test
    fun `委派路由随快照角色名派生且第二段挂真实角色id`() {
        // WHY：真实快照角色 id 为 UUID，写死 role-pm 等演示键会让委派路由到
        // 不存在的角色（气泡 senderRoleId 不在 roles 列表、名字退化为「角色」）；
        // 关键词必须随快照角色名派生，第二段气泡挂真实角色 id（评审 P6）。
        val uuidRole = SnapshotRole("role-uuid-pm", "产品经理", "target", "#4F46E5", "", "", 82, "moderate")
        // store.roles 门面从 dashboard.statuses 派生（SnapshotMapper.toRoleCards），
        // UUID 角色须同步落到 dashboard.statuses 才会出现在角色清单里
        val uuidStore = fixtureSnapshot().copy(
            roles = listOf(uuidRole),
            dashboard = fixtureSnapshot().dashboard.copy(
                statuses = listOf(
                    DashboardStatus("role-uuid-pm", "产品经理", "target", "#4F46E5", 82, 3,
                        "2026-08-25T07:50:00Z", false),
                ),
            ),
            conversations = fixtureSnapshot().conversations.filter { it.roleId == null },
        )
        val store = SnapshotStore().apply { applySnapshot(uuidStore) }
        val vm = newVm(store)

        vm.sendMessage("帮我把这件事同步给产品经理")
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(vm.uiState.value.messages.any { it.fromButler && it.text == "稍等，我让产品经理看一下。" })
        val roleBubble = vm.uiState.value.messages.firstOrNull { it.senderRoleId == "role-uuid-pm" }
            ?: error("委派第二段气泡缺失：未挂真实角色 id")
        // UUID 角色未命中演示回复表 → 角色名模板反馈（不以管家轮换回复冒充角色反馈）
        assertTrue(roleBubble.text.startsWith("来自产品经理的反馈"))
        assertTrue(roleBubble.text.contains("收到，这事我接下了"))
    }
}
