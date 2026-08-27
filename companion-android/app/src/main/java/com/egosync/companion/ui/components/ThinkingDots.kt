package com.egosync.companion.ui.components

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.egosync.companion.ui.theme.rememberReducedMotion

/**
 * 管家思考中三点指示：母本 ChatBubble.BounceDots——三圆点 160ms 交错弹跳（1.4s 周期）。
 * 自 ChatScreen 平移共享（组 6 onboarding 复用），逻辑零变更。
 */
@Composable
fun ThinkingDots(modifier: Modifier = Modifier) {
    // reduced-motion（系统「移除动画」开启）：三点静止显示，不弹跳
    if (rememberReducedMotion()) {
        Row(modifier, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            repeat(3) { ThinkingDot(yOffset = 0.dp) }
        }
        return
    }
    val transition = rememberInfiniteTransition(label = "dots")
    val phase by transition.animateFloat(
        initialValue = 0f,
        targetValue = 3f,
        animationSpec = infiniteRepeatable(tween(1400, easing = LinearEasing)),
        label = "dotPhase",
    )
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        repeat(3) { index ->
            // 各点错相 160ms（桌面 animationDelay 0/160/320ms）
            val t = ((phase - index * 0.8f).coerceIn(0f, 1f))
            val lift = if (t < 0.5f) t * 2 else (1f - t) * 2
            ThinkingDot(yOffset = (-6 * lift).dp)
        }
    }
}

@Composable
private fun ThinkingDot(yOffset: Dp) {
    Box(
        Modifier
            .size(8.dp)
            .offset(y = yOffset)
            .clip(CircleShape)
            .background(MaterialTheme.colorScheme.onSurfaceVariant)
    )
}
