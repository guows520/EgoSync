package com.egosync.companion.connection

import kotlin.math.min
import kotlin.random.Random

/**
 * 重连指数退避（AC3）：1s 起步、倍增、30s 封顶；抖动取 [delay/2, delay] 上半区
 * 均匀分布（P19 勘误：这是 half-jitter 区间而非 full jitter [0, delay]，首跳
 * 下限 500ms 属有意设计）——有界保证桌面恢复后必会重连，抖动防多设备同步重连风暴。
 */
class BackoffPolicy(
    private val random: Random = Random.Default,
    private val initialMs: Long = 1_000,
    private val maxMs: Long = 30_000,
) {
    private var attempt = 0

    /** 取下一次重连延迟并递增档位。 */
    fun nextDelayMs(): Long {
        attempt++
        var base = initialMs
        repeat(attempt - 1) {
            base = min(maxMs, base * 2)
        }
        if (base > maxMs) base = maxMs
        val half = base / 2
        return if (half <= 0) base else half + random.nextLong(half + 1)
    }

    /** 连接成功后归零（AC3「网络恢复后」回到初始档快速重连）。 */
    fun reset() {
        attempt = 0
    }
}
