package com.egosync.companion.connection

import android.content.SharedPreferences
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 配对状态持久化语义（AC6）：
 * - 未配对是出厂态——缺字段时必须如实呈现未配对，不得误报；
 * - 解除配对必须**清空全部**非机密元数据并回到未配对——残留任何一项
 *   （如桌面公钥）都会让重配对后的信任锚判断失真。
 */
class PairingStateStoreTest {

    private lateinit var prefs: FakeSharedPreferences
    private lateinit var store: PairingStateStore

    @Before
    fun setUp() {
        prefs = FakeSharedPreferences()
        store = PairingStateStore(prefs)
    }

    @Test
    fun fresh_install_reports_unpaired_with_no_metadata() {
        // WHY: AC6 重装重扫的前提——没有配对记录时不允许出现半旧的
        // 「已配对」状态，否则重装后的首跑流会被跳过。
        assertFalse(store.paired)
        assertNull(store.desktopPubkeyHex)
        assertNull(store.relayId)
        assertNull(store.relayAddr)
    }

    @Test
    fun save_persists_all_pairing_metadata() {
        store.save(
            desktopPubkeyHex = "ab".repeat(32),
            relayId = "cd".repeat(8),
            relayAddr = "ws://relay.example.com:7333",
        )
        assertTrue(store.paired)
        assertEquals("ab".repeat(32), store.desktopPubkeyHex)
        assertEquals("cd".repeat(8), store.relayId)
        assertEquals("ws://relay.example.com:7333", store.relayAddr)
    }

    @Test
    fun relay_addr_is_optional() {
        // WHY: 中继未部署时 QR 不带 relayAddr——空值语义必须原样保存，
        // 不能被默认值吞掉（AC4 的中继承载依赖该空值判断）。
        store.save(desktopPubkeyHex = "ab".repeat(32), relayId = "cd".repeat(8), relayAddr = null)
        assertTrue(store.paired)
        assertNull(store.relayAddr)
    }

    @Test
    fun clear_wipes_everything_back_to_unpaired() {
        store.save(desktopPubkeyHex = "ab", relayId = "cd", relayAddr = "ws://x")
        store.clear()
        assertFalse(store.paired)
        assertNull(store.desktopPubkeyHex)
        assertNull(store.relayId)
        assertNull(store.relayAddr)
    }
}

/** JVM 测试用内存版 SharedPreferences（android.jar stub 方法不可用，接口本身可实现）。 */
class FakeSharedPreferences : SharedPreferences {
    val map = mutableMapOf<String, Any?>()

    override fun getAll(): Map<String, *> = map.toMap()
    override fun getString(key: String, defValue: String?): String? = map[key] as? String ?: defValue
    override fun getStringSet(key: String, defValues: MutableSet<String>?): MutableSet<String>? =
        (map[key] as? Set<String>)?.toMutableSet() ?: defValues

    override fun getInt(key: String, defValue: Int): Int = map[key] as? Int ?: defValue
    override fun getLong(key: String, defValue: Long): Long = map[key] as? Long ?: defValue
    override fun getFloat(key: String, defValue: Float): Float = map[key] as? Float ?: defValue
    override fun getBoolean(key: String, defValue: Boolean): Boolean = map[key] as? Boolean ?: defValue
    override fun contains(key: String): Boolean = map.containsKey(key)

    override fun edit(): SharedPreferences.Editor = FakeEditor(map)
    override fun registerOnSharedPreferenceChangeListener(l: SharedPreferences.OnSharedPreferenceChangeListener?) = Unit
    override fun unregisterOnSharedPreferenceChangeListener(l: SharedPreferences.OnSharedPreferenceChangeListener?) = Unit

    private class FakeEditor(private val map: MutableMap<String, Any?>) : SharedPreferences.Editor {
        private val pending = mutableMapOf<String, Any?>()
        private val removals = mutableSetOf<String>()

        override fun putString(key: String, value: String?): SharedPreferences.Editor = apply { pending[key] = value }
        override fun putStringSet(key: String, values: MutableSet<String>?): SharedPreferences.Editor = apply { pending[key] = values }
        override fun putInt(key: String, value: Int): SharedPreferences.Editor = apply { pending[key] = value }
        override fun putLong(key: String, value: Long): SharedPreferences.Editor = apply { pending[key] = value }
        override fun putFloat(key: String, value: Float): SharedPreferences.Editor = apply { pending[key] = value }
        override fun putBoolean(key: String, value: Boolean): SharedPreferences.Editor = apply { pending[key] = value }
        override fun remove(key: String): SharedPreferences.Editor = apply { removals.add(key) }
        override fun clear(): SharedPreferences.Editor = apply { map.clear() }

        override fun commit(): Boolean {
            apply()
            return true
        }

        override fun apply() {
            removals.forEach { map.remove(it) }
            map.putAll(pending)
        }
    }
}
