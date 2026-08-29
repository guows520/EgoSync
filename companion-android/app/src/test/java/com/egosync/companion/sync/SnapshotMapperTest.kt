package com.egosync.companion.sync

import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * 快照时间格式化契约单测（13.2 评审 P11 行为锁定）：
 * - 跨日钟面按**系统时区**渲染（原实现硬编码 UTC，东八区用户会看到偏移 8 小时
 *   的钟面；相对差值按 epoch 计算不受影响）；
 * - formatDataCutoff 三分支（RFC3339/纯日期/非法）。
 */
class SnapshotMapperTest {

    @Test
    fun `相对时间在跨日后按系统时区渲染钟面`() {
        // WHY：桌面推的是 ISO UTC；用户看到的「8月20日 16:30」必须是本地钟面。
        // 期望值用同一 epoch 按系统时区现算——断言锁定「本地时区」意图本身，
        // 与 CI 机器时区无关
        val iso = "2026-08-20T16:30:00Z"
        val at = Instant.parse(iso)
        val local = at.atZone(ZoneId.systemDefault())
        // 距 now 超过 24h：走跨日钟面分支（相对差值用 epoch，任选足够远的 now）
        val nowMs = at.toEpochMilli() + 72L * 60 * 60 * 1000

        val label = SnapshotMapper.formatRelativeTimeIso(iso, nowMs)

        assertEquals(
            "%d月%d日 %02d:%02d".format(
                java.util.Locale.ROOT, local.monthValue, local.dayOfMonth, local.hour, local.minute,
            ),
            label,
        )
    }

    @Test
    fun `相对时间短距分支不涉时区`() {
        // WHY：刚刚/N分钟前/N小时前是 epoch 差值分支——此回归护栏确保时区改动
        // 未波及它们
        val iso = "2026-08-29T10:00:00Z"
        val base = Instant.parse(iso).toEpochMilli()
        assertEquals("刚刚", SnapshotMapper.formatRelativeTimeIso(iso, base + 30_000))
        assertEquals("5分钟前", SnapshotMapper.formatRelativeTimeIso(iso, base + 5 * 60_000))
        assertEquals("2小时前", SnapshotMapper.formatRelativeTimeIso(iso, base + 2L * 60 * 60_000))
    }

    @Test
    fun `相对时间null与非法输入原样降级`() {
        assertEquals("暂无活动", SnapshotMapper.formatRelativeTimeIso(null, 0))
        assertEquals("not-a-time", SnapshotMapper.formatRelativeTimeIso("not-a-time", 0))
    }

    @Test
    fun `formatDataCutoff三分支`() {
        // 纯日期分支无时分、与时区无关，可精确断言
        assertEquals("8月1日", SnapshotMapper.formatDataCutoff("2026-08-01"))
        // RFC3339 分支按系统时区现算期望值（锁定本地时区意图）
        val iso = "2026-08-20T16:30:00Z"
        val local = Instant.parse(iso).atZone(ZoneId.systemDefault())
        assertEquals(
            "%d月%d日 %02d:%02d".format(
                java.util.Locale.ROOT, local.monthValue, local.dayOfMonth, local.hour, local.minute,
            ),
            SnapshotMapper.formatDataCutoff(iso),
        )
        assertEquals("原样", SnapshotMapper.formatDataCutoff("原样"))
    }
}
