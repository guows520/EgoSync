package com.egosync.companion

import android.content.Context
import com.egosync.companion.command.CommandChannel
import com.egosync.companion.command.CommandSender
import com.egosync.companion.command.StreamCoordinator
import com.egosync.companion.connection.ConnectionClient
import com.egosync.companion.connection.PairingConnector
import com.egosync.companion.connection.RealConnectionClient
import com.egosync.companion.notify.InAppNotificationAdapter
import com.egosync.companion.notify.PrefsNoticeReadStateStore
import com.egosync.companion.sync.KeystoreSnapshotCipher
import com.egosync.companion.sync.QuickNoteQueue
import com.egosync.companion.sync.SnapshotCacheFile
import com.egosync.companion.sync.SnapshotFrameHandler
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.ui.theme.ThemeMode
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * 手工组装容器（无 DI 框架）：所有依赖的唯一持有处。
 * Story 12.4：[connection] 装配换为 [RealConnectionClient]（唯一换装点，
 * UX-M2）；[pairingConnector] 同实例二态（真实配对入口）。completePairing/
 * unpair 委托真实客户端（[PairingStateStore] 持久化真实配对态）。
 * Story 13.2：[snapshotStore] 换装为帧驱动实例（快照缓存 + Keystore 加密），
 * SNAPSHOT/STATE_DELTA 帧经 [SnapshotFrameHandler] 回灌，UI 层零改动。
 */
class AppModelContainer private constructor(context: Context) {

    private val prefs = context.getSharedPreferences("companion_prefs", Context.MODE_PRIVATE)

    /** 组件级协程作用域：先于连接层声明（[RealConnectionClient] 依赖注入）。 */
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    /** 快照数据源（声明先于连接层：frameConsumerFactory 闭包引用之）。
     *  缓存：filesDir 下 Keystore AES-GCM 包裹（离线呈现最后已知快照，AC3）。 */
    val snapshotStore = SnapshotStore(
        cache = SnapshotCacheFile(
            dir = context.filesDir,
            cipher = KeystoreSnapshotCipher(context),
        ),
        scope = scope,
    )

    /** 13.3 T5：流式聚合器（兼快照延后门——活跃流期间暂存，结束补应用）。 */
    val streamCoordinator = StreamCoordinator { snapshot, rawJson ->
        snapshotStore.applySnapshot(snapshot, rawJson)
    }

    /** 13.3 T5：指令通道（pending 表跨会话存活；RealConnectionClient 会话 pump 绑定）。 */
    val commandChannel = CommandChannel()

    private val realConnection = RealConnectionClient(
        context = context,
        scope = scope,
        // SNAPSHOT/STATE_DELTA 帧 → 分片重组 → 解析 → 流式门 → 快照全量替换（AC1/AC2）；
        // 新会话建立即解封快照通道（unpair 封存后重新配对，评审 P4）
        frameConsumerFactory = {
            snapshotStore.resume()
            SnapshotFrameHandler { snapshot, rawJson ->
                // 13.3：活跃流期间暂存（只留最新），流结束/失败立即补应用（Dev Notes §4 裁决）
                streamCoordinator.deliverSnapshot(snapshot, rawJson)
            }
        },
        // 降级态信息来自快照缓存可用性（FR-40，DebugConnectionMode.DEGRADED 同源）；
        // tick 传快照态流：缓存异步加载完成后驱动 Offline 重算（评审 P7）
        offlineInfo = { snapshotStore.offlineInfo() },
        offlineInfoTick = snapshotStore.state,
        commandChannel = commandChannel,
        streamCoordinator = streamCoordinator,
    )
    val connection: ConnectionClient = realConnection
    val pairingConnector: PairingConnector = realConnection

    /** 13.3 T5：指令发送入口（二态：真实客户端实现 / Preview fake 态 null → UI 显式失败提示）。 */
    val commandSender: CommandSender? = realConnection

    val quickNotes = QuickNoteQueue()
    val notifications = InAppNotificationAdapter(
        store = snapshotStore,
        scope = scope,
        // 本地已读记录持久化（评审决策 A）：STATE_DELTA 重发不复活已读
        readState = PrefsNoticeReadStateStore(
            context.getSharedPreferences("notice_read_state", Context.MODE_PRIVATE),
        ),
    )

    private val _themeMode = MutableStateFlow(
        if (prefs.getString(KEY_THEME, VALUE_DARK) == VALUE_LIGHT) ThemeMode.LIGHT else ThemeMode.DARK
    )
    val themeMode: StateFlow<ThemeMode> = _themeMode.asStateFlow()

    /** 一次性事件流（Snackbar）：SharedFlow 不做值去重——连续相同的错误
     *  各发一次（StateFlow 同值重发会被合并吞掉，评审 C21）。 */
    private val _events = kotlinx.coroutines.flow.MutableSharedFlow<String>(
        extraBufferCapacity = 16,
        onBufferOverflow = kotlinx.coroutines.channels.BufferOverflow.DROP_OLDEST,
    )
    val events: kotlinx.coroutines.flow.SharedFlow<String> = _events

    init {
        // FR-43：恢复连接后速记自动提交管家（mock：直接清队并提示）。
        // paired 守门：遮罩出口/自愈解除配对后的 Direct 并非真实恢复，
        // 不得触发 flush（FR-43 无丢失——真实恢复必然已配对）
        scope.launch {
            connection.state.collect { state ->
                if (state.engineAvailable && connection.paired.value) {
                    val pending = quickNotes.pendingCount()
                    if (pending > 0) {
                        quickNotes.flush()
                        _events.tryEmit("连接已恢复，$pending 条速记已提交管家处理")
                    }
                }
            }
        }
    }

    fun setThemeMode(mode: ThemeMode) {
        _themeMode.value = mode
        prefs.edit().putString(KEY_THEME, if (mode == ThemeMode.LIGHT) VALUE_LIGHT else VALUE_DARK).apply()
    }

    /** 委托真实客户端（配对持久化由 PairingStateStore 在配对成功时完成）。 */
    fun completePairing() {
        connection.completePairing()
    }

    /** FR-21：引导完成标记（镜像桌面 isFirstLaunch 语义；prefs 布尔等价）。 */
    fun isOnboarded(): Boolean = prefs.getBoolean(KEY_ONBOARDED, false)

    fun completeOnboarding() {
        prefs.edit().putBoolean(KEY_ONBOARDED, true).apply()
    }

    /** 委托真实客户端（清除信任锚与配对态、终止会话、复位 Debug 覆盖）；
     *  并清除本机快照缓存与内存态、通知与本地已读记录（配对解除后数据不再可信）。 */
    fun unpair() {
        connection.unpair()
        snapshotStore.clear()
        notifications.clear()
    }

    fun consumeEvent() {
        // 保留空实现（旧 StateFlow 语义遗留）：SharedFlow 事件无需手动消费
    }

    /** 一次性事件（Snackbar）——VM 层错误反馈经此直达全局提示（13.3）。 */
    fun showEvent(message: String) {
        _events.tryEmit(message)
    }

    companion object {
        @Volatile private var instance: AppModelContainer? = null

        /** 进程级单例：跨 Activity 配置重建（旋转等）保留速记队列与运行状态（FR-43 无丢失）。 */
        fun get(context: Context): AppModelContainer =
            instance ?: synchronized(this) {
                instance ?: AppModelContainer(context.applicationContext).also { instance = it }
            }

        private const val KEY_ONBOARDED = "onboarded"
        private const val KEY_THEME = "theme"
        const val VALUE_DARK = "dark"
        const val VALUE_LIGHT = "light"
    }
}
