package com.egosync.companion.pairing

import androidx.compose.ui.graphics.vector.ImageVector

import androidx.compose.material3.Icon
import com.egosync.companion.ui.icons.LucideIcons

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
    onScanCompleted: (String) -> Unit,
    onBack: () -> Unit,
    onEnterApp: () -> Unit,
    modifier: Modifier = Modifier,
    waitDesktopConfirm: Boolean = false,
    pairingError: String? = null,
    /** 扫码 QR 是否携带中继地址（12.5 AC1）：发现阶段行按中继配置如实渲染。 */
    qrHasRelay: Boolean = false,
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
                PairingStep.SCAN -> ScanStep(onScanCompleted, onBack, pairingError)
                PairingStep.CONNECTING -> ConnectingStep(connectStage, waitDesktopConfirm, qrHasRelay)
                PairingStep.SUCCESS -> SuccessStep(onEnterApp)
            }
            Spacer(Modifier.height(48.dp))
        }
    }
}

// ── ① 欢迎说明 ─────────────────────────────────────────────────────────

@Composable
private fun WelcomeStep(onStartScan: () -> Unit) {
    Icon(LucideIcons.Home, contentDescription = "管家", modifier = Modifier.size(32.dp))
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
    FeatureRow(LucideIcons.Home, "桌面唯一事实源", "手机是远程视图与指令入口，数据不离开你的电脑")
    Spacer(Modifier.height(18.dp))
    FeatureRow(LucideIcons.ShieldCheck, "端到端加密", "局域网直连优先，出网经中继转发且中继无法读取明文")
    Spacer(Modifier.height(18.dp))
    FeatureRow(LucideIcons.HardDrive, "诚实降级", "桌面离线时只读缓存 + 速记排队，恢复后自动补齐")
    Spacer(Modifier.height(56.dp))
    Button(onClick = onStartScan, modifier = Modifier.fillMaxWidth()) {
        Text("开始配对")
    }
}

@Composable
private fun FeatureRow(icon: ImageVector, title: String, body: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .background(MaterialTheme.colorScheme.surface)
            .padding(16.dp),
    ) {
        Icon(icon, contentDescription = null, modifier = Modifier.size(22.dp), tint = MaterialTheme.colorScheme.onSurface)
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

// ── ② 扫码取景（Story 12.4：真实相机扫码，AC1/裁决 2）────────────────

@Composable
private fun ScanStep(onScanCompleted: (String) -> Unit, onBack: () -> Unit, pairingError: String? = null) {
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
    if (pairingError != null) {
        // 配对失败原因如实呈现（Story 12.4）：否则用户只看到「退回扫码页」无从定位
        Spacer(Modifier.height(10.dp))
        Text(
            pairingError,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            textAlign = TextAlign.Center,
        )
    }
    Spacer(Modifier.height(40.dp))

    // 取景框：样式逐字保留原型（UX-M1），内容由模拟暗底换为真实相机预览
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(260.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(Color.Black.copy(alpha = 0.55f))
            .border(1.5.dp, MaterialTheme.colorScheme.outline, RoundedCornerShape(12.dp)),
        contentAlignment = Alignment.Center,
    ) {
        CameraPermissionGate {
            QrScannerView(
                modifier = Modifier.fillMaxSize(),
                onQrScanned = onScanCompleted,
            )
        }
        // 四角取景标记与底部提示绘制在相机之上（SurfaceView z-order）
        CornerMarks()
        Text(
            "将二维码对准取景框",
            style = MaterialTheme.typography.labelSmall,
            color = Color(0xFF9CA3AF),
            modifier = Modifier.align(Alignment.BottomCenter).padding(bottom = 14.dp),
        )
    }

    Spacer(Modifier.height(40.dp))
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
private fun ConnectingStep(stage: Int, waitDesktopConfirm: Boolean = false, qrHasRelay: Boolean = false) {
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
    if (waitDesktopConfirm) {
        // 待桌面确认（换绑，或中继首配确认门 12.5 AC2——手机侧无法区分两者，
        // 文案不得断言「已有配对设备」）：确认后手机将自动继续
        Spacer(Modifier.height(12.dp))
        Text(
            "请在桌面「设置 · 手机伴侣」确认配对请求，确认后手机将自动继续（120 秒内有效）…",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
    }
    Spacer(Modifier.height(36.dp))
    // 发现阶段行按中继配置如实渲染（12.5 AC1）：QR 含 relayAddr 时
    // 局域网发现失败会自动回退中继，不再断言「仅 NSD 局域网发现」
    ConnectStageRow(
        if (qrHasRelay) "发现桌面设备（局域网优先，中继兜底）" else "发现桌面设备（NSD 局域网发现）",
        stage >= 0,
    )
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
        Icon(LucideIcons.Check, contentDescription = null, modifier = Modifier.size(32.dp), tint = BrandGreen)
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
