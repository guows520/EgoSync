package com.egosync.companion.connection

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import java.io.IOException
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout

/** NSD 已解析端点（直连承载入口）。 */
data class NsdEndpoint(val host: String, val port: Int)

/**
 * NSD 发现的最小依赖面：连接编排层只依赖此接口（测试注入假实现即可 JVM
 * 全速验证编排逻辑，P7），NsdManager 真路径归 androidTest/手动冒烟。
 */
interface NsdDiscoverer {
    val resolved: SharedFlow<NsdEndpoint>

    /** 发现启动/解析失败信号（上层据此快速失败，不等整体超时）。 */
    val failed: SharedFlow<Unit>

    fun discover(expectedInstanceName: String)
    fun stopDiscovery()

    /**
     * 等待首个解析端点；发现/解析失败立即抛 [IOException]（P13：已知失败
     * 不得伪装成 12s 挂起），[timeoutMs] 为整体预算。
     */
    suspend fun awaitResolved(timeoutMs: Long): NsdEndpoint = withTimeout(timeoutMs) {
        coroutineScope {
            val failureWatch = launch { failed.collect { throw IOException("NSD 发现/解析失败") } }
            try {
                resolved.first()
            } finally {
                failureWatch.cancel()
            }
        }
    }
}

/**
 * NsdManager 发现（AC3）：服务类型 `_egosync._tcp.`、实例名 `EgoSync-{relayId 前 8}`、
 * TXT `proto=1`——桌面锚点见 companion_connection.rs（NSD_SERVICE_TYPE / register_nsd）。
 *
 * 只向 [resolved]/[failed] 转发本桌面实例的事件；发现生命周期与连接状态机协程对齐，
 * 停止/销毁必须 [stopDiscovery]（防 listener 泄漏）。
 *
 * P11：resolve 每次新建 listener 且全局单一未决（NsdManager 同一时刻只允许一个
 * resolve，并发第二次调用抛 IllegalArgumentException）——共享 listener 时代的
 * 「并发吞错」「stranded 永久卡死」不再存在；新发现轮次重置未决位，
 * 最坏退化为 fail-fast 重试而非永久静默。
 */
class NsdDiscovery(context: Context) : NsdDiscoverer, AutoCloseable {

    private val manager = context.getSystemService(Context.NSD_SERVICE) as NsdManager

    private val _resolved = MutableSharedFlow<NsdEndpoint>(
        replay = 0, extraBufferCapacity = 1, onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
    override val resolved: SharedFlow<NsdEndpoint> = _resolved

    private val _failed = MutableSharedFlow<Unit>(
        replay = 0, extraBufferCapacity = 1, onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
    override val failed: SharedFlow<Unit> = _failed

    @Volatile
    private var expectedInstance: String? = null

    private var discoveryListener: NsdManager.DiscoveryListener? = null

    /** NsdManager 约束：同一时刻仅允许一个未决 resolve（P11）。 */
    private val resolveOutstanding = AtomicBoolean(false)

    /** 启动发现（重复调用同一实例名为幂等）。 */
    override fun discover(expectedInstanceName: String) {
        if (expectedInstance == expectedInstanceName && discoveryListener != null) return
        stopDiscovery()
        expectedInstance = expectedInstanceName
        // 新发现轮次重置 resolve 未决位：stranded resolve（Android 偶发不回调）
        // 的最坏后果收敛为一次 fail-fast，而非此后所有发现永久静默（P11）
        resolveOutstanding.set(false)
        val listener = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(serviceType: String?) = Unit
            override fun onStartDiscoveryFailed(serviceType: String?, errorCode: Int) {
                // 失败即清位 + 快速上报（P13：不再静默吞掉让上层挂满 12s）
                discoveryListener = null
                _failed.tryEmit(Unit)
            }

            override fun onServiceFound(serviceInfo: NsdServiceInfo?) {
                val info = serviceInfo ?: return
                if (info.serviceName != expectedInstance) return
                if (!resolveOutstanding.compareAndSet(false, true)) return
                val resolveListener = object : NsdManager.ResolveListener {
                    override fun onResolveFailed(info: NsdServiceInfo?, errorCode: Int) {
                        resolveOutstanding.set(false)
                        _failed.tryEmit(Unit)
                    }

                    @Suppress("DEPRECATION")
                    override fun onServiceResolved(info: NsdServiceInfo?) {
                        resolveOutstanding.set(false)
                        val resolved = info ?: return
                        if (resolved.serviceName != expectedInstance) return
                        val proto = resolved.attributes[NSD_TXT_PROTO]?.toString(Charsets.UTF_8)
                        if (proto != NSD_TXT_PROTO_VALUE) return
                        val host = resolved.host?.hostAddress ?: return
                        _resolved.tryEmit(NsdEndpoint(host, resolved.port))
                    }
                }
                runCatching { manager.resolveService(info, resolveListener) }
                    .onFailure {
                        resolveOutstanding.set(false)
                        _failed.tryEmit(Unit)
                    }
            }

            override fun onServiceLost(serviceInfo: NsdServiceInfo?) = Unit

            override fun onDiscoveryStopped(serviceType: String?) = Unit
            override fun onStopDiscoveryFailed(serviceType: String?, errorCode: Int) = Unit
        }
        discoveryListener = listener
        manager.discoverServices(NSD_SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, listener)
    }

    override fun stopDiscovery() {
        val listener = discoveryListener ?: return
        discoveryListener = null
        runCatching { manager.stopServiceDiscovery(listener) }
    }

    override fun close() = stopDiscovery()

    private companion object {
        const val NSD_SERVICE_TYPE = "_egosync._tcp."
        const val NSD_TXT_PROTO = "proto"
        const val NSD_TXT_PROTO_VALUE = "1"
    }
}
