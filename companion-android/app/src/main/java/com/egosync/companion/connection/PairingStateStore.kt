package com.egosync.companion.connection

import android.content.SharedPreferences

/**
 * 配对非机密元数据存储（SharedPreferences）：desktop_pubkey_hex / relay_id /
 * relay_addr / paired 标志。私钥密文由 [PairingSecrets] 独立文件管理，不经此存储。
 *
 * 复用 AppModelContainer 既有的 companion_prefs 文件与 paired 键，保证既有
 * 首跑判定（KEY_PAIRED）与本存储是同一事实源，无迁移。
 */
class PairingStateStore(private val prefs: SharedPreferences) {

    val paired: Boolean get() = prefs.getBoolean(KEY_PAIRED, false)
    val desktopPubkeyHex: String? get() = prefs.getString(KEY_DESKTOP_PUBKEY, null)
    val relayId: String? get() = prefs.getString(KEY_RELAY_ID, null)
    val relayAddr: String? get() = prefs.getString(KEY_RELAY_ADDR, null)

    fun save(desktopPubkeyHex: String, relayId: String, relayAddr: String?) {
        prefs.edit()
            .putString(KEY_DESKTOP_PUBKEY, desktopPubkeyHex)
            .putString(KEY_RELAY_ID, relayId)
            .putString(KEY_RELAY_ADDR, relayAddr)
            .putBoolean(KEY_PAIRED, true)
            .apply()
    }

    /** 清空全部非机密元数据并回到未配对态（配对元数据残留会破坏重配对信任锚判断）。 */
    fun clear() {
        prefs.edit()
            .remove(KEY_DESKTOP_PUBKEY)
            .remove(KEY_RELAY_ID)
            .remove(KEY_RELAY_ADDR)
            .putBoolean(KEY_PAIRED, false)
            .apply()
    }

    companion object {
        const val PREFS_NAME = "companion_prefs"
        const val KEY_PAIRED = "paired"
        private const val KEY_DESKTOP_PUBKEY = "desktop_pubkey_hex"
        private const val KEY_RELAY_ID = "relay_id"
        private const val KEY_RELAY_ADDR = "relay_addr"
    }
}
