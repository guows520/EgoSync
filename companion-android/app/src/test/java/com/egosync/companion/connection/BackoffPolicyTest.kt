package com.egosync.companion.connection

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.random.Random

/**
 * 退避策略意图（AC3）：断线风暴下重连必须有界且有随机性——
 * 无界退避会让手机在桌面恢复后迟迟不重连；无抖动会让多设备同步重连打爆桌面。
 */
class BackoffPolicyTest {

    @Test
    fun first_attempt_is_half_jittered_initial_delay() {
        val policy = BackoffPolicy(random = Random(42))
        for (i in 1..100) {
            val delay = BackoffPolicy(random = Random(i.toLong())).nextDelayMs()
            assertTrue("首次退避应在 [500,1000] 内，实际 $delay", delay in 500..1000)
        }
    }

    @Test
    fun delay_grows_exponentially_and_caps_at_thirty_seconds() {
        val policy = BackoffPolicy(random = Random(7))
        val delays = (1..12).map { policy.nextDelayMs() }
        // 指数增长：1s→2s→4s→…→30s 封顶（区间含抖动）
        assertTrue(delays[0] <= 1_000)
        assertTrue(delays[1] in 1_000..2_000)
        assertTrue(delays[2] in 2_000..4_000)
        // 封顶后所有退避都落在 [15s, 30s]（full jitter：cap/2..cap）
        for (d in delays.drop(5)) {
            assertTrue("封顶后退避应 ≤30000，实际 $d", d <= 30_000)
            assertTrue("封顶后退避应 ≥15000（防零退避风暴），实际 $d", d >= 15_000)
        }
    }

    @Test
    fun reset_returns_to_initial_backoff() {
        val policy = BackoffPolicy(random = Random(9))
        repeat(8) { policy.nextDelayMs() }
        policy.reset()
        val delay = policy.nextDelayMs()
        assertTrue("reset 后首次退避应回到初始档，实际 $delay", delay in 500..1_000)
    }

    @Test
    fun jitter_is_actually_random_not_constant() {
        // WHY: 抖动非零是 AC3 明确要求——同档位多次取值必须分布开来，
        // 恒定值意味着抖动形同虚设。
        val policy = BackoffPolicy(random = Random(123))
        val capped = mutableSetOf<Long>()
        repeat(60) { capped.add(policy.nextDelayMs()) }
        repeat(200) { capped.add(BackoffPolicy(random = Random(it.toLong())).nextDelayMs()) }
        assertTrue("同档位退避值应发散，实际仅 ${capped.size} 种", capped.size > 10)
    }
}
