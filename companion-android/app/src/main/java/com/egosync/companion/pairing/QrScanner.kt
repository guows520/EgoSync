package com.egosync.companion.pairing

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.annotation.OptIn
import androidx.camera.core.CameraSelector
import androidx.camera.core.ExperimentalGetImage
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage

/**
 * 真实扫码（Story 12.4，AC1/裁决 2）：CameraX Preview + ML Kit BarcodeAnalysis。
 *
 * 样式零改动（UX-M1）由调用方 ScanStep 保证：本组件只填充取景框内容，
 * 取景框/四角标记 overlay 原样保留。
 *
 * 资源纪律（P4）：相机用例绑定/解绑与 ML Kit 扫描器释放均跟随本组合的
 * 生命周期——离开组合（进入 CONNECTING / 返回上一步）即全部归还。
 */
@Composable
fun QrScannerView(modifier: Modifier, onQrScanned: (String) -> Unit) {
    val lifecycleOwner = LocalLifecycleOwner.current
    // P10：识别锁存改为分析器内时间冷却——解析失败的 QR 不得永久堵死再扫
    val analyzer = remember { QrCodeAnalyzer(onQrScanned) }
    DisposableEffect(Unit) {
        onDispose { analyzer.close() }
    }
    var cameraProvider by remember { mutableStateOf<ProcessCameraProvider?>(null) }
    AndroidView(
        modifier = modifier,
        factory = { ctx ->
            val previewView = PreviewView(ctx).apply {
                scaleType = PreviewView.ScaleType.FILL_CENTER
            }
            val providerFuture = ProcessCameraProvider.getInstance(ctx)
            providerFuture.addListener(
                {
                    val provider = providerFuture.get()
                    cameraProvider = provider
                    val preview = Preview.Builder().build().also {
                        it.setSurfaceProvider(previewView.surfaceProvider)
                    }
                    val analysis = ImageAnalysis.Builder()
                        .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                        .build()
                        .also {
                            it.setAnalyzer(
                                ContextCompat.getMainExecutor(ctx),
                                analyzer,
                            )
                        }
                    try {
                        provider.unbindAll()
                        provider.bindToLifecycle(
                            lifecycleOwner,
                            CameraSelector.DEFAULT_BACK_CAMERA,
                            preview,
                            analysis,
                        )
                    } catch (_: Exception) {
                        // 相机被占用/不可用：取景框保持可见，用户可返回或重试
                    }
                },
                ContextCompat.getMainExecutor(ctx),
            )
            previewView
        },
    )
    // P4：离开组合即解绑相机——取景、分析、隐私指示灯不得挂满整个配对流
    DisposableEffect(cameraProvider) {
        onDispose { cameraProvider?.unbindAll() }
    }
}

/**
 * ML Kit QR 分析器（P10）：以时间冷却替代一次性 AtomicBoolean 锁存——
 * 残缺 QR 解析失败后 2s 内即可重扫，同一码仍不会高频重复回调。
 * （NFR-M7：QR 内容不入日志。）
 */
private class QrCodeAnalyzer(private val onQr: (String) -> Unit) : ImageAnalysis.Analyzer {

    private val scanner = BarcodeScanning.getClient(
        BarcodeScannerOptions.Builder()
            .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
            .build(),
    )

    @Volatile
    private var lastAcceptedAtMs = 0L

    @OptIn(ExperimentalGetImage::class)
    override fun analyze(imageProxy: ImageProxy) {
        val mediaImage = imageProxy.image
        if (mediaImage == null || System.currentTimeMillis() - lastAcceptedAtMs < RESCAN_COOLDOWN_MS) {
            imageProxy.close()
            return
        }
        val input = InputImage.fromMediaImage(mediaImage, imageProxy.imageInfo.rotationDegrees)
        scanner.process(input)
            .addOnSuccessListener { barcodes ->
                val raw = barcodes.firstOrNull()?.rawValue ?: return@addOnSuccessListener
                val now = System.currentTimeMillis()
                if (now - lastAcceptedAtMs >= RESCAN_COOLDOWN_MS) {
                    lastAcceptedAtMs = now
                    onQr(raw)
                }
            }
            .addOnCompleteListener { imageProxy.close() }
    }

    /** P4：ML Kit 原生扫描器随扫码视图离开组合显式释放（native 资源不靠 GC）。 */
    fun close() {
        scanner.close()
    }

    private companion object {
        /** 同码冷却窗口：防重复回调，又保证解析失败后 2s 内可重扫（P10）。 */
        const val RESCAN_COOLDOWN_MS = 2_000L
    }
}

/**
 * 相机权限闸门：授权后渲染 [content]（扫码视图），拒绝态显示中文引导。
 * 权限拒绝不是终态——用户可随时重试（AC1 权限拒绝态引导文案）。
 */
@Composable
fun CameraPermissionGate(content: @Composable () -> Unit) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    var granted by rememberSaveable {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) ==
                PackageManager.PERMISSION_GRANTED,
        )
    }
    val launcher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted = it }

    LaunchedEffect(Unit) {
        if (!granted) launcher.launch(Manifest.permission.CAMERA)
    }

    // P21：用户在系统设置授权后返回前台，拒绝态必须随 ON_RESUME 自动解除，
    // 不得要求用户再点一次「重新授权相机」
    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                granted = ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) ==
                    PackageManager.PERMISSION_GRANTED
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }

    if (granted) {
        content()
    } else {
        Column(modifier = Modifier.fillMaxSize()) {
            Spacer(Modifier.height(48.dp))
            Text(
                "需要相机权限才能扫码配对",
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onSurface,
                modifier = Modifier.padding(horizontal = 16.dp),
            )
            Spacer(Modifier.height(8.dp))
            Text(
                "二维码来自桌面端「设置 · 手机伴侣」。请在系统弹窗中允许相机，或在系统设置中手动开启。",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 16.dp),
            )
            Spacer(Modifier.height(16.dp))
            Button(
                onClick = { launcher.launch(Manifest.permission.CAMERA) },
                modifier = Modifier.padding(horizontal = 16.dp),
            ) {
                Text("重新授权相机")
            }
        }
    }
}
