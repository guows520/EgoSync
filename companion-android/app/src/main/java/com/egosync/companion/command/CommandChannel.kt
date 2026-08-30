package com.egosync.companion.command

import com.egosync.companion.connection.Frame
import com.egosync.companion.connection.FrameCodec
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.withTimeoutOrNull
import org.json.JSONObject

/**
 * 指令发送接口（Story 13.3 T5）：UI 层唯一发送入口。
 *
 * - [RealConnectionClient][com.egosync.companion.connection.RealConnectionClient]
 *   实现之（委托注入的 [CommandChannel]）；fake/debug 态无实现 → VM 显式
 *   失败提示，不伪造回执（NFR-M3）。
 * - 会话不可用/超时/桌面错误统一抛 [CommandException]。
 */
interface CommandSender {
    /** 会话是否在线（出站通道已绑定）。 */
    val sessionActive: Boolean

    /**
     * 发送指令并等待 COMMAND_RESULT；成功返回 result JSON，失败抛 [CommandException]。
     * 幂等语义由桌面按 commandId 保证——同一语义指令重发（新 commandId）会重复执行，
     * 重放需复用 commandId（14.1 速记队列的底座，本 story 不消费）。
     */
    suspend fun execute(action: String, paramsJson: String = "{}"): JSONObject
}

/** ack 看门狗默认预算（毫秒）。评审裁决 A：60s——chat.send 走 DB 写 +
 * opencode 会话创建，慢链路下 15s 会伪超时（用户重发新 commandId → 桌面
 * 重复执行非幂等指令）；真断连由 unbind 快速失败兜底，不等满超时。 */
internal const val DEFAULT_ACK_TIMEOUT_MS = 60_000L

/**
 * 指令通道（Story 13.3 T5）：
 * - 出站：指令帧经会话 pump 协程编码发送（[bind] 由会话循环调用——每会话
 *   绑定一次，镜像桌面 outbound 模式）；
 * - 入站：`Frame.CommandResult` 经 [onCommandResult] 完成对应 pending；
 * - **pending 表跨会话存活**（勿放每会话重建的消费者）——会话断开时
 *   [unbind] 若仍是当前绑定则全部失败（连接断开错误），在途结果可经
 *   新会话由桌面幂等补发（单槽会话语义，Dev Notes §3）。
 *
 * NFR-M7：不打印指令 JSON/结果内容，只记 action 与计数。
 */
class CommandChannel(
    private val ackTimeoutMs: Long = DEFAULT_ACK_TIMEOUT_MS,
) {
    /** 出站帧队列（容量 256：指令为用户触发低频操作，溢出由看门狗兜底）。 */
    internal class Binding {
        val outbound = Channel<Frame>(capacity = 256)
    }

    @Volatile
    private var binding: Binding? = null

    private val pending = ConcurrentHashMap<String, CompletableDeferred<CommandAck>>()

    val sessionActive: Boolean
        get() = binding != null

    /**
     * 会话建立时绑定出站通道，返回该会话的 pump 源（会话循环内消费并
     * `session.send(FrameCodec.encode(frame, session.transport))`）。
     * 覆盖旧绑定时关闭其出站通道：旧会话 pump 的 for-in 循环正常退出，
     * 不悬挂泄漏；旧通道排队帧随旧会话消亡（在途指令由 ack 看门狗兜底）。
     */
    internal fun bind(): Binding {
        val b = Binding()
        binding?.let { old ->
            old.outbound.close()
            com.egosync.companion.connection.CompanionLog.warn(TAG, "新会话绑定，旧出站通道关闭")
        }
        binding = b
        return b
    }

    /**
     * 会话结束时解绑：仅当仍是当前绑定时清空并失败全部 pending（新会话
     * 已接管则保留——桌面把在途结果发往当前槽，ack 可达）。
     */
    internal fun unbind(b: Binding) {
        if (binding === b) {
            binding = null
            b.outbound.close()
            failAllPending("连接已断开，指令未送达桌面")
        }
    }

    /** 会话彻底不可用（unpair 等）：清空并失败全部 pending。 */
    fun shutdown() {
        binding?.outbound?.close()
        binding = null
        failAllPending("连接已断开，指令未送达桌面")
    }

    suspend fun execute(action: String, paramsJson: String): org.json.JSONObject {
        val b = binding
            ?: throw CommandException("ConnectionError", "桌面引擎不可达")
        val commandId = UUID.randomUUID().toString()
        val deferred = CompletableDeferred<CommandAck>()
        pending[commandId] = deferred
        try {
            val envelope = CommandEnvelope.encode(commandId, action, paramsJson)
            // TOCTOU 整改：注册 pending 之后复查绑定——unbind 若发生在注册
            // 之前，此处立即失败（failAllPending 清不到未注册的条目）；若发生
            // 在注册之后则已被 failAllPending 命中，此处复查同样兜住。
            if (binding !== b) {
                throw CommandException("ConnectionError", "连接已切换，指令未送达")
            }
            // 超时同时覆盖 send 与 await（整改：send 挂起在满通道/死 pump 上
            // 时，仅包 await 的看门狗永不启动）；withTimeoutOrNull 不偷换外层
            // 调用方的取消/超时信号（裸 catch TimeoutCancellationException 会
            // 把外层超时误转成 ConnectionError 向上抛）。
            val ack = withTimeoutOrNull(ackTimeoutMs) {
                b.outbound.send(Frame.Command(envelope))
                deferred.await()
            } ?: throw CommandException("ConnectionError", "指令超时未收到回执（$action）")
            return ack.resultOrThrow()
        } finally {
            pending.remove(commandId)
        }
    }

    /** 帧循环收到 COMMAND_RESULT：按 commandId 完成等待方（未知 id 忽略——重放/迟到回执）。 */
    fun onCommandResult(data: String) {
        val ack = CommandAck.parse(data) ?: run {
            com.egosync.companion.connection.CompanionLog.warn(TAG, "COMMAND_RESULT 解析失败，跳过")
            return
        }
        if (ack.commandId.isEmpty()) {
            // 桌面 envelope 解析失败的回执（commandId=""）——留痕不静默：
            // 否则发送方只能等超时且排障无从区分「桌面拒绝」与「回执丢失」。
            com.egosync.companion.connection.CompanionLog.warn(TAG, "回执缺少 commandId，无法关联在途指令")
            return
        }
        pending.remove(ack.commandId)?.let { it.complete(ack) }
    }

    private fun failAllPending(message: String) {
        val error = CommandException("ConnectionError", message)
        val entries = pending.entries.toList()
        pending.clear()
        for ((_, deferred) in entries) {
            deferred.complete(
                CommandAck(commandId = "", ok = false, result = null, error = error),
            )
        }
        if (entries.isNotEmpty()) {
            com.egosync.companion.connection.CompanionLog.warn(TAG, "会话断开，${entries.size} 条在途指令失败")
        }
    }

    private companion object {
        const val TAG = "Companion/Command"
    }
}
