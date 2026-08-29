package com.egosync.companion

import android.content.Context
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

    private val realConnection = RealConnectionClient(
        context = context,
        scope = scope,
        // SNAPSHOT/STATE_DELTA 帧 → 分片重组 → 解析 → 快照全量替换（AC1/AC2）；
        // 新会话建立即解封快照通道（unpair 封存后重新配对，评审 P4）
        frameConsumerFactory = {
            snapshotStore.resume()
            SnapshotFrameHandler { snapshot, rawJson ->
                snapshotStore.applySnapshot(snapshot, rawJson)
            }
        },
        // 降级态信息来自快照缓存可用性（FR-40，DebugConnectionMode.DEGRADED 同源）；
        // tick 传快照态流：缓存异步加载完成后驱动 Offline 重算（评审 P7）
        offlineInfo = { snapshotStore.offlineInfo() },
        offlineInfoTick = snapshotStore.state,
    )
    val connection: ConnectionClient = realConnection
    val pairingConnector: PairingConnector = realConnection

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

    /** 一次性事件消息（Snackbar），UI 消费后调 [consumeEvent]。 */
    private val _eventMessage = MutableStateFlow<String?>(null)
    val eventMessage: StateFlow<String?> = _eventMessage.asStateFlow()

    init {
        // FR-43：恢复连接后速记自动提交管家（mock：直接清队并提示）
        scope.launch {
            connection.state.collect { state ->
                if (state.engineAvailable) {
                    val pending = quickNotes.pendingCount()
                    if (pending > 0) {
                        quickNotes.flush()
                        _eventMessage.value = "连接已恢复，$pending 条速记已提交管家处理"
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
        _eventMessage.value = null
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
