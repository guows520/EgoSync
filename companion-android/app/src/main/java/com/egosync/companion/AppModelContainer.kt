package com.egosync.companion

import android.content.Context
import com.egosync.companion.connection.ConnectionState
import com.egosync.companion.connection.FakeConnectionClient
import com.egosync.companion.notify.InAppNotificationAdapter
import com.egosync.companion.sync.QuickNoteQueue
import com.egosync.companion.ui.theme.ThemeMode
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * 手工组装容器（无 DI 框架）：所有 Fake/Mock 单例的唯一持有处。
 * 接入真实连接层时，只需替换 [connection] 的实现——其余代码零改动。
 */
class AppModelContainer private constructor(context: Context) {

    private val prefs = context.getSharedPreferences("companion_prefs", Context.MODE_PRIVATE)

    val connection = FakeConnectionClient(initialPaired = prefs.getBoolean(KEY_PAIRED, false))
    val quickNotes = QuickNoteQueue()
    val notifications = InAppNotificationAdapter()

    private val _themeMode = MutableStateFlow(
        if (prefs.getString(KEY_THEME, VALUE_DARK) == VALUE_LIGHT) ThemeMode.LIGHT else ThemeMode.DARK
    )
    val themeMode: StateFlow<ThemeMode> = _themeMode.asStateFlow()

    /** 一次性事件消息（Snackbar），UI 消费后调 [consumeEvent]。 */
    private val _eventMessage = MutableStateFlow<String?>(null)
    val eventMessage: StateFlow<String?> = _eventMessage.asStateFlow()

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

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

    fun completePairing() {
        connection.completePairing()
        prefs.edit().putBoolean(KEY_PAIRED, true).apply()
    }

    fun unpair() {
        connection.unpair()
        prefs.edit().putBoolean(KEY_PAIRED, false).apply()
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

        private const val KEY_PAIRED = "paired"
        private const val KEY_THEME = "theme"
        const val VALUE_DARK = "dark"
        const val VALUE_LIGHT = "light"
    }
}
