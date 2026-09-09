package com.egosync.companion.command

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import org.json.JSONObject

/**
 * 指令通道测试替身（Story 13.3 T9）：记录调用序并可编排响应/异常。
 * [handler] 为空时一律回空 JSON（成功 ack 语义由 VM 各自断言）。
 * T-S10：透传记录 commandId（待发箱重发幂等断言）。
 */
class FakeCommandSender(
    var handler: (suspend (action: String, params: JSONObject) -> JSONObject)? = null,
) : CommandSender {

    override val sessionActive: StateFlow<Boolean> = MutableStateFlow(true)

    /** (action, params) 调用序。 */
    val calls = mutableListOf<Pair<String, JSONObject>>()

    /** chat.send 重发时透传的 commandId（幂等重放断言用；普通发送为 null）。 */
    val passedCommandIds = mutableListOf<String?>()

    /** 各 action 最近一次入参。 */
    fun paramsOf(action: String): JSONObject? =
        calls.lastOrNull { it.first == action }?.second

    override suspend fun execute(action: String, paramsJson: String, commandId: String?): JSONObject {
        val params = JSONObject(paramsJson)
        calls += action to params
        passedCommandIds += commandId
        return handler?.invoke(action, params) ?: JSONObject()
    }
}
