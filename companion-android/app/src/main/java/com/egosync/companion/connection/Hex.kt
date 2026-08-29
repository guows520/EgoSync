package com.egosync.companion.connection

/**
 * 十六进制编码（连接层内用：信任锚公钥比对）。
 * 放在这里而非依赖 okio，保持连接层依赖面最小。
 */
internal object Hex {
    private const val DIGITS = "0123456789abcdef"

    fun encode(bytes: ByteArray): String {
        val sb = StringBuilder(bytes.size * 2)
        for (b in bytes) {
            val v = b.toInt() and 0xFF
            sb.append(DIGITS[v ushr 4]).append(DIGITS[v and 0x0F])
        }
        return sb.toString()
    }
}
