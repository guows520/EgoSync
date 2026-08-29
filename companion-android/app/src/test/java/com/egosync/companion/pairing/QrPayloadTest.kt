package com.egosync.companion.pairing

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * QR payload 解析契约——与桌面 models/companion.rs::QrPayload serde camelCase
 * 输出逐字段一致。这是配对信任链的入口：字段缺失或畸形必须整体拒绝，
 * 绝不能带病进入握手（否则信任锚校验基于残缺数据）。
 */
class QrPayloadTest {

    @Test
    fun parses_full_camel_case_payload_from_desktop() {
        // WHY: 桌面 serde camelCase 是三端 QR 契约的字面形态——桌面侧
        // relay_addr 已配置时 QR 带 relayAddr，手机侧必须原样取回。
        val qr = QrPayload.parse(
            """{"relayAddr":"ws://relay.example.com:7333",""" +
                """"desktopStaticPubkey":"${"ab".repeat(32)}",""" +
                """"relayId":"${"cd".repeat(8)}","pairingNonce":"${"0f".repeat(16)}"}""",
        )
        assertEquals("ws://relay.example.com:7333", qr?.relayAddr)
        assertEquals("ab".repeat(32), qr?.desktopStaticPubkey)
        assertEquals("cd".repeat(8), qr?.relayId)
        assertEquals("0f".repeat(16), qr?.pairingNonce)
    }

    @Test
    fun relay_addr_null_means_relay_disabled() {
        // WHY: 桌面 V1 未部署中继时 relayAddr 为 null（serde Option 形态）——
        // 空值语义必须保留：中继承载禁用，离网即 Offline（AC4 的判断依据）。
        val qr = QrPayload.parse(
            """{"relayAddr":null,"desktopStaticPubkey":"${"ab".repeat(32)}",""" +
                """"relayId":"${"cd".repeat(8)}","pairingNonce":"${"0f".repeat(16)}"}""",
        )
        assertNull(qr?.relayAddr)
    }

    @Test
    fun missing_any_required_field_is_rejected() {
        // WHY: 缺 desktopStaticPubkey 的 QR 若被接受，配对将把一个未知密钥
        // 当作信任锚——宁拒不配，不接受残缺信任链。
        assertNull(
            QrPayload.parse(
                """{"relayAddr":null,"relayId":"${"cd".repeat(8)}","pairingNonce":"${"0f".repeat(16)}"}""",
            ),
        )
        assertNull(QrPayload.parse("""{"desktopStaticPubkey":"${"ab".repeat(32)}"}"""))
        assertNull(QrPayload.parse("不是 JSON"))
    }

    @Test
    fun malformed_pubkey_or_relay_id_length_or_charset_is_rejected() {
        // WHY（P15）: 信任锚是 64-hex 静态公钥、relayId 是 16-hex——残缺/非 hex
        // 的「公钥」若放行，要么在握手上才失败、要么以垃圾数据当信任锚比对；
        // 契约校验必须在入口整体拒绝（此前 "aa" 两字符都能通过）。
        assertNull("2 字符短公钥必须拒绝", QrPayload.parse(
            """{"desktopStaticPubkey":"aa","relayId":"${"cd".repeat(8)}","pairingNonce":"n"}""",
        ))
        assertNull("64 字符非 hex 必须拒绝", QrPayload.parse(
            """{"desktopStaticPubkey":"${"zz".repeat(32)}","relayId":"${"cd".repeat(8)}","pairingNonce":"n"}""",
        ))
        assertNull("63 字符公钥必须拒绝", QrPayload.parse(
            """{"desktopStaticPubkey":"${"a".repeat(63)}","relayId":"${"cd".repeat(8)}","pairingNonce":"n"}""",
        ))
        assertNull("relayId 长度违约必须拒绝", QrPayload.parse(
            """{"desktopStaticPubkey":"${"ab".repeat(32)}","relayId":"cd","pairingNonce":"n"}""",
        ))
    }
}
