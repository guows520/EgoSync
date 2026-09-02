package com.egosync.companion.pairing

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.ImageFormat
import android.hardware.Camera
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Box
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
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.egosync.companion.connection.CompanionLog
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.abs

/**
 * 真实扫码（案件 android-pairing-black-viewfinder 裁决）：Camera1
 * （android.hardware.Camera）+ SurfaceView 可见预览 + NV21 预览帧喂 ML Kit。
 *
 * CameraX/Camera2 在卓易通容器 open 静默失败（OPENING→CLOSING→CLOSED、零帧零
 * 错误），Camera1 已在同机验证可出帧——扫码相机固定走 Camera1，不再并行
 * 绑定 CameraX。
 *
 * 样式零改动（UX-M1）由调用方 ScanStep 保证：本组件只填充取景框内容，
 * 取景框/四角标记 overlay 原样保留。
 *
 * 资源纪律（P4）：相机是单一持有者——进入组合随 surface 就绪打开，离开组合
 * （进入 CONNECTING / 返回上一步 / ON_STOP）即清回调、停预览并 release。
 */
@Composable
fun QrScannerView(
    modifier: Modifier,
    onQrScanned: (String) -> Unit,
) {
    val currentOnQrScanned by rememberUpdatedState(onQrScanned)
    var cameraError by remember { mutableStateOf<String?>(null) }
    val scanner = remember {
        Camera1QrScanner(
            onQrScanned = { raw -> currentOnQrScanned(raw) },
            onCameraError = { message -> cameraError = message },
        )
    }

    DisposableEffect(scanner) {
        onDispose { scanner.release() }
    }

    // P4/隐私指示灯：ON_STOP 释放相机，回前台重开（对齐 CameraX bindToLifecycle
    // 的行为，避免应用后台仍持有摄像头）
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner, scanner) {
        val observer = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_STOP -> scanner.stopForBackground()
                Lifecycle.Event.ON_RESUME -> scanner.resumeFromBackground()
                else -> Unit
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }

    Box(modifier = modifier) {
        if (cameraError == null) {
            AndroidView(
                modifier = Modifier.fillMaxSize(),
                factory = { context ->
                    SurfaceView(context).also { view ->
                        view.holder.addCallback(scanner)
                        view.display?.let { scanner.updateDisplayRotation(it.rotation) }
                    }
                },
                update = { view ->
                    view.display?.let { scanner.updateDisplayRotation(it.rotation) }
                },
            )
        } else {
            // CAMERA_OPEN_FAILURE：显式中文失败提示（不黑屏、不崩溃），
            // 由取景框下方既有「返回上一步」退出重试
            Text(
                cameraError.orEmpty(),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(horizontal = 16.dp),
            )
        }
    }
}

/**
 * QR 回调冷却闸门（JVM 可单测）：冷却窗口内只放行一次成功解码——相机持续
 * 出帧时同一码不得高频重复触发 onQrScanned；窗口过期后放行，保证残缺
 * payload 被 ViewModel 拒绝后仍可重新扫码（P10）。
 */
internal class QrCallbackGate(
    private val now: () -> Long = System::currentTimeMillis,
) {
    private var lastAcceptedAtMs: Long? = null

    fun tryAccept(): Boolean {
        val current = now()
        val last = lastAcceptedAtMs
        return if (last == null || current - last >= RESCAN_COOLDOWN_MS) {
            lastAcceptedAtMs = current
            true
        } else {
            false
        }
    }

    companion object {
        /** 同码冷却窗口：防重复回调，又保证解析失败后 2s 内可重扫（P10）。 */
        const val RESCAN_COOLDOWN_MS = 2_000L
    }
}

/**
 * Camera1 扫码持有者：真实可见预览（SurfaceView + setPreviewDisplay）与
 * NV21 预览帧分析共用同一 Camera 实例。所有相机操作固定在内部 HandlerThread
 * 上执行，预览回调也回到该线程——不在 UI 线程碰 Camera，也不阻塞预览。
 */
private class Camera1QrScanner(
    private val onQrScanned: (String) -> Unit,
    private val onCameraError: (String) -> Unit,
) : SurfaceHolder.Callback {

    private val thread = HandlerThread("qr-camera").apply { start() }
    private val handler = Handler(thread.looper)
    private val mainHandler = Handler(Looper.getMainLooper())

    /** ML Kit 扫描器（P4：随 release() 显式释放，native 资源不靠 GC）。 */
    private val qrScanner = BarcodeScanning.getClient(
        BarcodeScannerOptions.Builder()
            .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
            .build(),
    )

    @Volatile
    private var released = false

    @Volatile
    private var backgroundStopped = false

    @Volatile
    private var failed = false

    @Volatile
    private var holder: SurfaceHolder? = null

    @Volatile
    private var surfaceWidth = 0

    @Volatile
    private var surfaceHeight = 0

    @Volatile
    private var surfaceGeneration = 0L

    @Volatile
    private var displayRotationDegrees = 0

    // 以下仅在相机线程访问
    private var camera: Camera? = null
    private var cameraSurfaceHolder: SurfaceHolder? = null
    private var cameraSurfaceGeneration = -1L
    private var cameraSurfaceWidth = 0
    private var cameraSurfaceHeight = 0
    private var sensorOrientation = 0
    private var previewWidth = 0
    private var previewHeight = 0
    private var previewBufferSize = 0

    /** 上一帧仍在 ML Kit 解析时跳过新帧：不排队、不阻塞预览线程。 */
    private val analyzing = AtomicBoolean(false)

    private val gate = QrCallbackGate()

    fun updateDisplayRotation(rotation: Int) {
        displayRotationDegrees = rotation * 90
    }

    fun stopForBackground() {
        backgroundStopped = true
        surfaceGeneration += 1
        handler.post { teardownCamera() }
    }

    fun resumeFromBackground() {
        if (released || failed) return
        backgroundStopped = false
        val boundHolder = holder ?: return
        val width = surfaceWidth
        val height = surfaceHeight
        val generation = surfaceGeneration
        if (width <= 0 || height <= 0) return
        handler.post { openCamera(boundHolder, width, height, generation) }
    }

    fun release() {
        if (released) return
        released = true
        backgroundStopped = true
        surfaceGeneration += 1
        val boundHolder = holder
        holder = null
        surfaceWidth = 0
        surfaceHeight = 0
        boundHolder?.removeCallback(this)
        handler.removeCallbacksAndMessages(null)
        handler.post {
            teardownCamera()
            qrScanner.close()
            thread.quitSafely()
        }
    }

    // ── SurfaceHolder.Callback（主线程）─────────────────────────────

    override fun surfaceCreated(holder: SurfaceHolder) {
        this.holder = holder
        // surface 尚未确定尺寸——等 surfaceChanged 再绑定相机
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        this.holder = holder
        this.surfaceWidth = width
        this.surfaceHeight = height
        val generation = ++surfaceGeneration
        if (backgroundStopped || failed) return
        handler.post { openCamera(holder, width, height, generation) }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        if (this.holder === holder) {
            this.holder = null
            surfaceWidth = 0
            surfaceHeight = 0
            surfaceGeneration += 1
        }
        handler.post { teardownCamera() }
    }

    // ── 相机线程──────────────────────────────────────────────────────

    private fun openCamera(
        holder: SurfaceHolder,
        surfaceWidth: Int,
        surfaceHeight: Int,
        generation: Long,
    ) {
        if (released || backgroundStopped || failed) return
        if (this.holder !== holder || surfaceGeneration != generation) return
        
        // Surface 重建或尺寸变化：先归还旧实例再重开
        if (camera != null) {
            teardownCamera()
        }
        
        try {
            val cameraId = findBackCameraId() ?: run {
                fail("未检测到后置摄像头，无法扫码")
                return
            }
            val camera = Camera.open(cameraId)
            this.camera = camera
            cameraSurfaceHolder = holder
            cameraSurfaceGeneration = generation
            cameraSurfaceWidth = surfaceWidth
            cameraSurfaceHeight = surfaceHeight
            
            val info = Camera.CameraInfo()
            Camera.getCameraInfo(cameraId, info)
            sensorOrientation = info.orientation
            camera.setDisplayOrientation(rotationDegrees())

            val params = camera.parameters
            val size = choosePreviewSize(params, surfaceWidth, surfaceHeight)
                ?: error("相机未提供可用预览尺寸")
            params.setPreviewSize(size.width, size.height)
            params.previewFormat = ImageFormat.NV21
            if (params.supportedFocusModes.contains(Camera.Parameters.FOCUS_MODE_CONTINUOUS_PICTURE)) {
                params.focusMode = Camera.Parameters.FOCUS_MODE_CONTINUOUS_PICTURE
            }
            camera.parameters = params
            previewWidth = size.width
            previewHeight = size.height
            previewBufferSize = previewWidth * previewHeight * 3 / 2

            camera.setPreviewDisplay(holder)
            camera.setPreviewCallback { data, _ -> onPreviewFrame(data) }
            camera.startPreview()
        } catch (e: Exception) {
            teardownCamera()
            CompanionLog.warn("Pairing", "Camera1 打开失败: ${e.javaClass.simpleName}")
            fail("相机启动失败，请返回上一步后重试")
        }
    }

    /** 扫码始终用后摄（Always 约束）：无后摄时显式失败，不静默改前摄。 */
    private fun findBackCameraId(): Int? {
        val numberOfCameras = Camera.getNumberOfCameras()
        for (index in 0 until numberOfCameras) {
            val info = Camera.CameraInfo()
            Camera.getCameraInfo(index, info)
            if (info.facing == Camera.CameraInfo.CAMERA_FACING_BACK) return index
        }
        return null
    }

    /**
     * 预览档位选择：Camera1 尺寸是横向（width>height）；setDisplayOrientation(90)
     * 旋转后显示为竖向，所以用 surfaceWidth/surfaceHeight 匹配横向预览比；
     * 限制在 1280x720 内避免大帧拖慢 ML Kit。
     */
    private fun choosePreviewSize(
        params: Camera.Parameters,
        surfaceWidth: Int,
        surfaceHeight: Int,
    ): Camera.Size? {
        val all = params.supportedPreviewSizes
        if (all.isNullOrEmpty()) return params.previewSize
        val targetRatio = if (surfaceWidth > 0 && surfaceHeight > 0) {
            surfaceWidth.toFloat() / surfaceHeight
        } else {
            4f / 3f
        }
        val capped = all.filter {
            it.width <= MAX_PREVIEW_WIDTH && it.height <= MAX_PREVIEW_HEIGHT
        }
        return (capped.ifEmpty { all }).minByOrNull { size ->
            val ratioError = abs(size.width.toFloat() / size.height - targetRatio)
            val areaError = abs(size.width * size.height - PREFERRED_PREVIEW_AREA).toFloat() /
                PREFERRED_PREVIEW_AREA
            ratioError + areaError * 0.5f
        }
    }

    /** 后摄 NV21 帧转正（顺时针）：预览显示方向与 ML Kit 旋转元数据同源。 */
    private fun rotationDegrees(): Int = (sensorOrientation - displayRotationDegrees + 360) % 360

    private fun onPreviewFrame(data: ByteArray) {
        if (released || data.isEmpty()) return
        if (!analyzing.compareAndSet(false, true)) return
        val frame = data.copyOf()
        val input = InputImage.fromByteArray(
            frame,
            previewWidth,
            previewHeight,
            rotationDegrees(),
            ImageFormat.NV21,
        )
        qrScanner.process(input)
            .addOnSuccessListener { barcodes ->
                // NFR-M7：QR 内容不入日志；rawValue 只流向回调
                val raw = barcodes.firstOrNull()?.rawValue
                if (raw != null && gate.tryAccept()) {
                    mainHandler.post { if (!released) onQrScanned(raw) }
                }
            }
            .addOnFailureListener { e ->
                CompanionLog.warn("Pairing", "ML Kit 解码失败: ${e.javaClass.simpleName}")
            }
            .addOnCompleteListener { analyzing.set(false) }
    }

    /** 幂等：清回调 → 停预览 → release（release 必须尽力执行，防相机泄漏）。 */
    private fun teardownCamera() {
        val camera = this.camera ?: return
        this.camera = null
        try {
            camera.setPreviewCallback(null)
            camera.stopPreview()
        } catch (e: Exception) {
            CompanionLog.warn("Pairing", "Camera1 停止预览异常: ${e.javaClass.simpleName}")
        }
        try {
            camera.release()
        } catch (e: Exception) {
            CompanionLog.warn("Pairing", "Camera1 release 异常: ${e.javaClass.simpleName}")
        }
    }

    private fun fail(message: String) {
        failed = true
        teardownCamera()
        mainHandler.post { onCameraError(message) }
    }

    private companion object {
        const val MAX_PREVIEW_WIDTH = 1280
        const val MAX_PREVIEW_HEIGHT = 720
        const val PREFERRED_PREVIEW_AREA = 640 * 480
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
