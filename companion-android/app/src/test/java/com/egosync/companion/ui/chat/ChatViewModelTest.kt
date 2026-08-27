package com.egosync.companion.ui.chat

import com.egosync.companion.sync.ChatMessage
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
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 多会话状态机意图验证（规格 FR-3 / FR-6，I/O 契约六场景的 VM 侧语义）。
 *
 * 参考 PairingViewModelTest 的 StandardTestDispatcher 模式：
 * VM 内所有 delay（思考/工具阶段/打字机）都由测试调度器推进，
 * 因此可以用 advanceTimeBy 精确停在「流式进行中」这一中间态来验证中断守卫。
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

    private fun newVm() = ChatViewModel(SnapshotStore)

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
        val historyConv = vm.uiState.value.conversations
            .first { it.title == SnapshotStore.butlerHistoryConversationTitle }
        assertEquals(SnapshotStore.initialChat, vm.uiState.value.messages)

        vm.selectConversation(historyConv.id)
        assertEquals(SnapshotStore.butlerHistoryChat, vm.uiState.value.messages)
        assertEquals(historyConv.id, vm.uiState.value.currentConversationId)

        // 切回当前种子会话，消息流恢复
        val currentConv = vm.uiState.value.conversations
            .first { it.title == SnapshotStore.butlerCurrentConversationTitle }
        vm.selectConversation(currentConv.id)
        assertEquals(SnapshotStore.initialChat, vm.uiState.value.messages)
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

        assertEquals(SnapshotStore.roleChatSeeds.getValue("role-pm"), vm.uiState.value.messages)
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

        val okrId = vm.uiState.value.conversations
            .first { it.title == SnapshotStore.butlerHistoryConversationTitle }.id
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
        val okrId = vm.uiState.value.conversations
            .first { it.title == SnapshotStore.butlerHistoryConversationTitle }.id
        vm.selectConversation(okrId)

        vm.selectRole("role-pm")
        vm.selectRole(null)

        assertEquals(okrId, vm.uiState.value.currentConversationId)
        assertEquals(SnapshotStore.butlerHistoryChat, vm.uiState.value.messages)
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
        val okrConv = vm.uiState.value.conversations
            .first { it.title == SnapshotStore.butlerHistoryConversationTitle }

        vm.deleteConversation(vm.uiState.value.currentConversationId)

        assertEquals(okrConv.id, vm.uiState.value.currentConversationId)
        assertEquals(SnapshotStore.butlerHistoryChat, vm.uiState.value.messages)
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
        val okrId = vm.uiState.value.conversations
            .first { it.title == SnapshotStore.butlerHistoryConversationTitle }.id

        vm.deleteConversation(okrId)

        assertEquals(currentId, vm.uiState.value.currentConversationId)
        assertEquals(SnapshotStore.initialChat, vm.uiState.value.messages)
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
        val okrId = vm.uiState.value.conversations
            .first { it.title == SnapshotStore.butlerHistoryConversationTitle }.id

        vm.sendMessage("帮我看看明天的安排")
        dispatcher.scheduler.advanceTimeBy(1800)
        assertTrue(vm.uiState.value.responding)

        vm.selectConversation(okrId)

        assertFalse(vm.uiState.value.responding)
        assertEquals(SnapshotStore.butlerHistoryChat, vm.uiState.value.messages)

        dispatcher.scheduler.advanceUntilIdle()
        // 目标会话保持原样：种子消息一条不多一条不少
        assertEquals(SnapshotStore.butlerHistoryChat, vm.uiState.value.messages)
    }
}
