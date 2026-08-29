package com.egosync.companion.pairing

import org.json.JSONException
import org.json.JSONObject

/**
 * 配对二维码 payload——与桌面 models/companion.rs::QrPayload serde camelCase
 * 输出逐字段一致（三端 QR 契约的字面形态）。
 *
 * `relayAddr` 为 null = 桌面未部署中继（serde Option 形态）：中继承载禁用，
 * 离网即 Offline（AC4 判断依据）。
 */
data class QrPayload(
    val relayAddr: String?,
    val desktopStaticPubkey: String,
    val relayId: String,
    val pairingNonce: String,
) {
    companion object {
        /** 任一必填字段缺失/格式非法即整体拒绝，绝不允许带病进入握手（信任链完整性）。 */
        fun parse(json: String): QrPayload? = try {
            val obj = JSONObject(json)
            val relayAddr = obj.optString("relayAddr").takeIf { it.isNotBlank() }
            val pubkey = obj.optString("desktopStaticPubkey")
            val relayId = obj.optString("relayId")
            val nonce = obj.optString("pairingNonce")
            // P15：形状校验升级为契约校验——桌面产出 64-hex 静态公钥 / 16-hex
            // relayId，残缺扫描件必须在此拒之门外（而非到信任锚比对才失败）
            if (isHexLen(pubkey, PUBKEY_HEX_LEN) && isHexLen(relayId, RELAY_ID_HEX_LEN) && nonce.isNotBlank()) {
                QrPayload(relayAddr, pubkey, relayId, nonce)
            } else {
                null
            }
        } catch (_: JSONException) {
            null
        }

        private fun isHexLen(value: String, len: Int): Boolean =
            value.length == len && value.all { it in '0'..'9' || it in 'a'..'f' || it in 'A'..'F' }

        /** 桌面 QrPayload 契约：X25519 公钥 32 字节 = 64 hex；relay_id = hex(SHA256(pubkey))[..16]。 */
        private const val PUBKEY_HEX_LEN = 64
        private const val RELAY_ID_HEX_LEN = 16
    }
}
