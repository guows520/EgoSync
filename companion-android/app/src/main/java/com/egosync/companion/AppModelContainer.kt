package com.egosync.companion

import android.content.Context
import com.egosync.companion.command.CommandChannel
import com.egosync.companion.command.CommandSender
import com.egosync.companion.command.StreamCoordinator
import com.egosync.companion.connection.ConnectionClient
import com.egosync.companion.connection.PairingConnector
import com.egosync.companion.connection.PairingRecoveryReason
import com.egosync.companion.connection.RealConnectionClient
import com.egosync.companion.notify.InAppNotificationAdapter
import com.egosync.companion.notify.PrefsNoticeReadStateStore
import com.egosync.companion.sync.ChatOutbox
import com.egosync.companion.sync.ChatOutboxFile
import com.egosync.companion.sync.KeystoreSnapshotCipher
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
    private val snapshotCipher = KeystoreSnapshotCipher(context)

    val snapshotStore = SnapshotStore(
        cache = SnapshotCacheFile(
            dir = context.filesDir,
            cipher = snapshotCipher,
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

    /**
     * T-S10 对话离线待发箱：!commandReady 发送入队（FIFO + 幂等 commandId），
     * 队列变更同步落盘（Keystore 加密 + 原子写，进程被杀不丢）；flush 守门
     * commandReady && paired 由 ChatViewModel 判定（不私建平行判据）。
     */
    val chatOutbox = ChatOutbox(
        persistence = ChatOutboxFile(dir = context.filesDir, cipher = snapshotCipher),
    )

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
        // T-S10：离线待发箱 flush 由 ChatViewModel 驱动（守门 commandReady && paired
        // 在 VM 内合并判定并串行重发，SPEC state-model §4.4/§4.5）；容器只负责
        // 构造时从磁盘恢复队列（ChatOutbox init）——旧 mock 收集器随全屏遮罩退役
        // 删除（离线录入无丢失语义由对话离线待发箱真实承接，SPEC supersessions.md）。

        // T-S4：配对/凭据失效恢复——客户端已完成原子序列前半（会话/指令/流式
        // 失效、清配对态、擦密钥）；容器补后半：清快照缓存与通知（配对解除后
        // 数据不再可信，镜像 unpair 清理面）+ 原因提示。待发箱保留（FR-43 无
        // 丢失，重新配对后续发）。导航回配对流由 AppNavHost 订阅同一事件执行。
        scope.launch {
            realConnection.pairingRecovery.collect { reason ->
                snapshotStore.clear()
                notifications.clear()
                _events.tryEmit(recoveryMessage(reason))
            }
        }
    }

    fun setThemeMode(mode: ThemeMode) {        _themeMode.value = mode
        prefs.edit().putString(KEY_THEME, if (mode == ThemeMode.LIGHT) VALUE_LIGHT else VALUE_DARK).apply()
    }

    /** 委托真实客户端（配对持久化由 PairingStateStore 在配对成功时完成）。 */
    fun completePairing() {
        connection.completePairing()
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

    /** T-S4：恢复原因 → 用户文案（SPEC qr-semantics §4 语义，文案层映射）。 */
    private fun recoveryMessage(reason: PairingRecoveryReason): String = when (reason) {
        PairingRecoveryReason.CredentialMissing -> "本机配对密钥缺失，已清除配对态，请重新扫码配对"
        PairingRecoveryReason.CredentialInvalid -> "本机配对密钥已失效，已清除配对态，请重新扫码配对"
        PairingRecoveryReason.PairingRevoked -> "桌面已解除与此手机的配对，请重新扫码配对"
        PairingRecoveryReason.TrustMismatch -> "桌面身份已变化（可能重装），需重新配对"
    }

    companion object {
        @Volatile private var instance: AppModelContainer? = null

        /** 进程级单例：跨 Activity 配置重建（旋转等）保留待发箱与运行状态（FR-43 无丢失）。 */
        fun get(context: Context): AppModelContainer =
            instance ?: synchronized(this) {
                instance ?: AppModelContainer(context.applicationContext).also { instance = it }
            }

        private const val KEY_THEME = "theme"
        const val VALUE_DARK = "dark"
        const val VALUE_LIGHT = "light"
    }
}
