package com.egosync.companion.pairing

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.ui.theme.BrandGreen
import com.egosync.companion.ui.theme.BrandIndigoLight
import com.egosync.companion.ui.theme.EgoSyncTheme

/**
 * 首跑配对流（全屏，无底栏）：
 * 欢迎说明 → 扫码取景模拟 → 连接中动画 → 配对成功。
 */
@Composable
fun PairingScreen(
    step: PairingStep,
    connectStage: Int,
    onStartScan: () -> Unit,
    onScanCompleted: () -> Unit,
    onBack: () -> Unit,
    onEnterApp: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(modifier = modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .systemBarsPadding()
                .padding(horizontal = 28.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Spacer(Modifier.height(64.dp))
            when (step) {
                PairingStep.WELCOME -> WelcomeStep(onStartScan)
                PairingStep.SCAN -> ScanStep(onScanCompleted, onBack)
                PairingStep.CONNECTING -> ConnectingStep(connectStage)
                PairingStep.SUCCESS -> SuccessStep(onEnterApp)
            }
            Spacer(Modifier.height(48.dp))
        }
    }
}

// ── ① 欢迎说明 ─────────────────────────────────────────────────────────

@Composable
private fun WelcomeStep(onStartScan: () -> Unit) {
    Text("🤵", style = MaterialTheme.typography.displaySmall)
    Spacer(Modifier.height(20.dp))
    Text(
        "EgoSync 伴侣",
        style = MaterialTheme.typography.headlineSmall,
        color = MaterialTheme.colorScheme.onBackground,
    )
    Spacer(Modifier.height(10.dp))
    Text(
        "你桌面上的老管家，现在与你同行。",
        style = MaterialTheme.typography.bodyLarge,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        textAlign = TextAlign.Center,
    )
    Spacer(Modifier.height(48.dp))
    FeatureRow("🏠", "桌面唯一事实源", "手机是远程视图与指令入口，数据不离开你的电脑")
    Spacer(Modifier.height(18.dp))
    FeatureRow("🔐", "端到端加密", "局域网直连优先，出网经中继转发且中继无法读取明文")
    Spacer(Modifier.height(18.dp))
    FeatureRow("🪨", "诚实降级", "桌面离线时只读缓存 + 速记排队，恢复后自动补齐")
    Spacer(Modifier.height(56.dp))
    Button(onClick = onStartScan, modifier = Modifier.fillMaxWidth()) {
        Text("开始配对")
    }
}

@Composable
private fun FeatureRow(icon: String, title: String, body: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .background(MaterialTheme.colorScheme.surface)
            .padding(16.dp),
    ) {
        Text(icon, style = MaterialTheme.typography.titleLarge)
        Spacer(Modifier.size(14.dp))
        Column {
            Text(title, style = MaterialTheme.typography.titleMedium)
            Spacer(Modifier.height(3.dp))
            Text(
                body,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ── ② 扫码取景模拟 ─────────────────────────────────────────────────────

@Composable
private fun ScanStep(onScanCompleted: () -> Unit, onBack: () -> Unit) {
    Text(
        "扫码配对",
        style = MaterialTheme.typography.headlineSmall,
        color = MaterialTheme.colorScheme.onBackground,
    )
    Spacer(Modifier.height(8.dp))
    Text(
        "请对准桌面端 EgoSync「设置 · 手机伴侣」中展示的二维码",
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        textAlign = TextAlign.Center,
    )
    Spacer(Modifier.height(40.dp))

    // 取景框（纯模拟，无相机）：静息框 + 缓慢扫掠线
    val transition = rememberInfiniteTransition(label = "scanline")
    val scanY by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(2200, easing = LinearEasing), RepeatMode.Restart),
        label = "scanY",
    )
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(260.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(Color.Black.copy(alpha = 0.55f))
            .border(1.5.dp, MaterialTheme.colorScheme.outline, RoundedCornerShape(12.dp)),
        contentAlignment = Alignment.Center,
    ) {
        // 四角取景标记
        CornerMarks()
        // 扫掠线
        Box(
            Modifier
                .fillMaxWidth()
                .padding(horizontal = 22.dp)
                .height(2.dp)
                .background(
                    Brush.horizontalGradient(
                        listOf(Color.Transparent, BrandIndigoLight, Color.Transparent)
                    )
                )
                .offset(y = (scanY * 236).dp),
        )
        Text(
            "原型模拟：点击下方按钮完成扫码",
            style = MaterialTheme.typography.labelSmall,
            color = Color(0xFF9CA3AF),
            modifier = Modifier.align(Alignment.BottomCenter).padding(bottom = 14.dp),
        )
    }

    Spacer(Modifier.height(40.dp))
    Button(onClick = onScanCompleted, modifier = Modifier.fillMaxWidth()) {
        Text("模拟扫码成功")
    }
    TextButton(onClick = onBack, modifier = Modifier.padding(top = 6.dp)) {
        Text("返回上一步")
    }
}

@Composable
private fun BoxScope.CornerMarks() {
    val corner = 22.dp
    val thickness = 3.dp
    val color = BrandIndigoLight
    // 左上 / 右上 / 左下 / 右下
    Box(
        Modifier
            .align(Alignment.TopStart)
            .padding(14.dp)
            .size(corner, thickness)
            .background(color)
    )
    Box(
        Modifier
            .align(Alignment.TopStart)
            .padding(14.dp)
            .size(thickness, corner)
            .background(color)
    )
    Box(
        Modifier
            .align(Alignment.TopEnd)
            .padding(14.dp)
            .size(corner, thickness)
            .background(color)
    )
    Box(
        Modifier
            .align(Alignment.TopEnd)
            .padding(14.dp)
            .size(thickness, corner)
            .background(color)
    )
    Box(
        Modifier
            .align(Alignment.BottomStart)
            .padding(14.dp)
            .size(corner, thickness)
            .background(color)
    )
    Box(
        Modifier
            .align(Alignment.BottomStart)
            .padding(14.dp)
            .size(thickness, corner)
            .background(color)
    )
    Box(
        Modifier
            .align(Alignment.BottomEnd)
            .padding(14.dp)
            .size(corner, thickness)
            .background(color)
    )
    Box(
        Modifier
            .align(Alignment.BottomEnd)
            .padding(14.dp)
            .size(thickness, corner)
            .background(color)
    )
}

// ── ③ 连接中动画 ───────────────────────────────────────────────────────

@Composable
private fun ConnectingStep(stage: Int) {
    Spacer(Modifier.height(56.dp))
    CircularProgressIndicator(
        color = MaterialTheme.colorScheme.primary,
        strokeWidth = 3.dp,
    )
    Spacer(Modifier.height(32.dp))
    Text(
        "正在与桌面端建立加密通道…",
        style = MaterialTheme.typography.titleMedium,
        color = MaterialTheme.colorScheme.onBackground,
    )
    Spacer(Modifier.height(36.dp))
    ConnectStageRow("发现桌面设备（NSD 局域网发现）", stage >= 0)
    Spacer(Modifier.height(14.dp))
    ConnectStageRow("交换密钥（Noise XX 握手）", stage >= 1)
    Spacer(Modifier.height(14.dp))
    ConnectStageRow("验证双方身份", stage >= 2)
}

@Composable
private fun ConnectStageRow(text: String, active: Boolean) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier.fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .background(
                if (active) MaterialTheme.colorScheme.primaryContainer
                else MaterialTheme.colorScheme.surface
            )
            .padding(horizontal = 16.dp, vertical = 12.dp),
    ) {
        Box(
            Modifier
                .size(10.dp)
                .clip(CircleShape)
                .background(if (active) BrandGreen else MaterialTheme.colorScheme.outline)
        )
        Spacer(Modifier.size(12.dp))
        Text(
            text,
            style = MaterialTheme.typography.bodyMedium,
            color = if (active) MaterialTheme.colorScheme.onSurface
            else MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

// ── ④ 配对成功 ─────────────────────────────────────────────────────────

@Composable
private fun SuccessStep(onEnterApp: () -> Unit) {
    Spacer(Modifier.height(48.dp))
    Box(
        Modifier
            .size(88.dp)
            .clip(CircleShape)
            .background(BrandGreen.copy(alpha = 0.16f)),
        contentAlignment = Alignment.Center,
    ) {
        Text("✓", style = MaterialTheme.typography.displaySmall, color = BrandGreen)
    }
    Spacer(Modifier.height(28.dp))
    Text(
        "配对成功",
        style = MaterialTheme.typography.headlineSmall,
        color = MaterialTheme.colorScheme.onBackground,
    )
    Spacer(Modifier.height(10.dp))
    Text(
        "管家已在您的桌面端待命。\n从现在起，您的角色、任务与记忆将实时同步到这部手机。",
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        textAlign = TextAlign.Center,
    )
    Spacer(Modifier.height(48.dp))
    Button(onClick = onEnterApp, modifier = Modifier.fillMaxWidth()) {
        Text("进入主界面")
    }
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun PairingWelcomePreview() {
    EgoSyncTheme {
        PairingScreen(
            step = PairingStep.WELCOME,
            connectStage = 0,
            onStartScan = {}, onScanCompleted = {}, onBack = {}, onEnterApp = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun PairingScanPreview() {
    EgoSyncTheme {
        PairingScreen(
            step = PairingStep.SCAN,
            connectStage = 0,
            onStartScan = {}, onScanCompleted = {}, onBack = {}, onEnterApp = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun PairingConnectingPreview() {
    EgoSyncTheme {
        PairingScreen(
            step = PairingStep.CONNECTING,
            connectStage = 1,
            onStartScan = {}, onScanCompleted = {}, onBack = {}, onEnterApp = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun PairingSuccessPreview() {
    EgoSyncTheme {
        PairingScreen(
            step = PairingStep.SUCCESS,
            connectStage = 2,
            onStartScan = {}, onScanCompleted = {}, onBack = {}, onEnterApp = {},
        )
    }
}
