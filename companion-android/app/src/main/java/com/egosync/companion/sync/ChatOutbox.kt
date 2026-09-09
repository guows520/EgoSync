package com.egosync.companion.sync

import com.egosync.companion.connection.CompanionLog
import java.io.File
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONArray
import org.json.JSONObject

/**
 * 离线待发箱条目（SPEC state-and-recovery-model §4.4）。
 *
 * - [commandId]：UUID v4，**跨重发稳定**——重发复用同一 commandId，桌面
 *   dispatcher 幂等缓存命中返回首次结果（网络重传/竞窗重发零重复执行）；
 * - [conversationId]：null = 本地占位会话（chat.send 不传，桌面新建并回填
 *   真实 id，规则同 sendMessage 现状）；
 * - [localMessageId]：待发气泡 id——进程重建后按此还原待发态气泡（幂等去重键）。
 */
data class OutboxEntry(
    val commandId: String,
    val roleId: String?,
    val conversationId: String?,
    val content: String,
    val localMessageId: String,
)

/**
 * 对话离线待发箱（T-S10，用户裁决：队列化 + 落盘）。
 *
 * - FIFO 内存队列（[entries]）+ 队列变更同步落盘（入队/删条目即持久化）；
 * - 进程被杀/冷启动后构造时从 [persistence] 恢复，队列与待发态不丢；
 * - **不观察连接状态**（SPEC 统一状态模型贯穿声明：flush 守门
 *   commandReady && paired 由消费方——ChatViewModel——判定，待发箱不自建平行判据）。
 */
class ChatOutbox(
    private val persistence: ChatOutboxPersistence? = null,
) {

    private val _entries = MutableStateFlow<List<OutboxEntry>>(emptyList())
    val entries: StateFlow<List<OutboxEntry>> = _entries.asStateFlow()

    init {
        // 启动恢复：落盘队列原样还原（含本地占位会话条目——conversationId=null）
        persistence?.load()?.let { restored ->
            _entries.value = restored
            if (restored.isNotEmpty()) {
                CompanionLog.info("Companion/Outbox", "离线待发箱恢复 ${restored.size} 条")
            }
        }
    }

    /** 入队（FIFO 尾插）并同步落盘。 */
    fun enqueue(entry: OutboxEntry) {
        _entries.value = _entries.value + entry
        persist()
    }

    /** 删条目（发送成功/业务错误路径）并同步落盘。 */
    fun remove(commandId: String) {
        val next = _entries.value.filterNot { it.commandId == commandId }
        if (next.size != _entries.value.size) {
            _entries.value = next
            persist()
        }
    }

    private fun persist() {
        persistence?.save(_entries.value)
    }

    companion object {
        /** 条目 ↔ JSON（落盘格式版本 1）。 */
        internal fun entryToJson(entry: OutboxEntry): JSONObject = JSONObject()
            .put("commandId", entry.commandId)
            .put("roleId", entry.roleId ?: JSONObject.NULL)
            .put("conversationId", entry.conversationId ?: JSONObject.NULL)
            .put("content", entry.content)
            .put("localMessageId", entry.localMessageId)

        internal fun entryFromJson(obj: JSONObject): OutboxEntry? = try {
            OutboxEntry(
                commandId = obj.getString("commandId"),
                roleId = obj.optString("roleId").ifEmpty { null },
                conversationId = obj.optString("conversationId").ifEmpty { null },
                content = obj.getString("content"),
                localMessageId = obj.getString("localMessageId"),
            )
        } catch (e: Exception) {
            null // 单条形状损坏跳过（fail-soft），不整箱拒载
        }
    }
}

/** 待发箱落盘缝（JVM 单测注入假实现即可全速验证，P7 手法）。 */
interface ChatOutboxPersistence {
    fun save(entries: List<OutboxEntry>)
    fun load(): List<OutboxEntry>
}

/**
 * 待发箱落盘文件（`chat_outbox.bin`）：布局 = 4 字节大端版本号 + IV（12B）‖
 * AES-GCM 密文（条目 JSON 数组）。镜像 [SnapshotCacheFile]：复用
 * [KeystoreSnapshotCipher] 加密（与快照缓存同级保护，SPEC 裁决 B）、
 * 原子写（tmp + rename）、密文/结构损坏自愈（删除重建空队——条目可由
 * 用户重发重建，不静默不炸进程）。
 */
class ChatOutboxFile(
    private val dir: File,
    private val cipher: SnapshotCipher,
) : ChatOutboxPersistence {

    override fun save(entries: List<OutboxEntry>) {
        val array = JSONArray()
        entries.forEach { array.put(ChatOutbox.entryToJson(it)) }
        val blob = byteArrayOf(
            (OUTBOX_VERSION ushr 24).toByte(),
            (OUTBOX_VERSION ushr 16).toByte(),
            (OUTBOX_VERSION ushr 8).toByte(),
            OUTBOX_VERSION.toByte(),
        ) + cipher.encrypt(array.toString().toByteArray(Charsets.UTF_8))
        atomicWrite(outboxFile(), blob)
    }

    override fun load(): List<OutboxEntry> {
        val file = outboxFile()
        if (!file.exists()) return emptyList()
        val blob = file.readBytes()
        if (blob.size < VERSION_HEADER_LEN) return selfHeal("待发箱文件过短（${blob.size} 字节）")
        val version = ((blob[0].toInt() and 0xFF) shl 24) or
            ((blob[1].toInt() and 0xFF) shl 16) or
            ((blob[2].toInt() and 0xFF) shl 8) or
            (blob[3].toInt() and 0xFF)
        if (version != OUTBOX_VERSION) {
            return selfHeal("待发箱版本未知：$version（本端支持 $OUTBOX_VERSION）")
        }
        return try {
            val json = String(cipher.decrypt(blob.copyOfRange(VERSION_HEADER_LEN, blob.size)), Charsets.UTF_8)
            val array = JSONArray(json)
            (0 until array.length()).mapNotNull { ChatOutbox.entryFromJson(array.getJSONObject(it)) }
        } catch (e: Exception) {
            // 密钥失效/密文损坏：删残留自愈（空队重建；与快照缓存 P14 同款语义）
            selfHeal("待发箱密文不可解（${e.javaClass.simpleName}）")
        }
    }

    private fun selfHeal(reason: String): List<OutboxEntry> {
        outboxFile().delete()
        CompanionLog.warn("Companion/Outbox", "待发箱自愈删除：$reason")
        return emptyList()
    }

    /**
     * 原子落盘：先写临时文件再原子重命名（照抄 SnapshotCacheFile.atomicWrite）。
     * 写失败/改名失败均清理 tmp 残留；调用方（ChatOutbox）串行化变更。
     */
    private fun atomicWrite(target: File, bytes: ByteArray) {
        val tmp = File(dir, "$OUTBOX_FILE.tmp")
        try {
            tmp.writeBytes(bytes)
            if (!tmp.renameTo(target)) {
                throw java.io.IOException("待发箱落盘失败")
            }
        } finally {
            tmp.delete() // 改名成功后已不存在，删除为无害空操作
        }
    }

    private fun outboxFile(): File = File(dir, OUTBOX_FILE)

    private companion object {
        const val OUTBOX_VERSION = 1
        const val OUTBOX_FILE = "chat_outbox.bin"
        const val VERSION_HEADER_LEN = 4
    }
}
