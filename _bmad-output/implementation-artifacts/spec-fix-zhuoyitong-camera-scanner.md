---
title: '修复卓易通环境扫码取景黑屏'
type: 'bugfix'
created: '2026-05-28'
status: 'in-review'
baseline_commit: '1c6147bf830bc224df657c2cdeeaeb8e0bf40809'
context:
  - '{project-root}/_bmad-output/project-context.md'

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 在 HarmonyOS 6.1 + 卓易通 1.0.10.75 真机上，CameraX/Camera2 绑定成功但相机状态从 OPENING 直接转 CLOSING/CLOSED，PreviewView 无画面；同机其他安卓应用可正常使用相机，当前配对扫码因此黑屏且无法继续。

**Approach:** 将扫码相机从 CameraX/Camera2 改为已验证可在该环境出帧的 Camera1（`android.hardware.Camera`）路径；使用真实预览 Surface 显示后摄画面，并把 Camera1 预览帧交给现有 ML Kit QR 解析。保留现有配对页布局、权限门、二维码 payload 校验与回调契约。

## Boundaries & Constraints

**Always:** 使用后摄进行扫码；相机生命周期随扫码页进入/离开而启动/释放；每个 `Image`/预览缓冲及时释放或复用；QR 内容不得写日志；解析成功只回调一次并由现有 ViewModel 接管；适配卓易通的同时不破坏标准 Android 真机；保留现有 UI 样式与权限拒绝引导。

**Ask First:** 若 Camera1 在标准 Android 构建或可用 API 级别上无法满足 ML Kit 输入要求，先暂停并报告，不降级为未经验证的替代扫码库。

**Never:** 不保留 CameraX 作为并行相机占用者；不保留临时 `INVESTIGATE-TMP` 诊断文字、无显示探针、前摄实验或静默吞异常；不改配对协议、连接状态机、桌面端服务与非扫码 UI；不把 QR 原文写入日志。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| HAPPY_PATH | 已授予 CAMERA，扫码页可见，后摄出帧且画面含有效桌面 QR | 取景框显示后摄画面；ML Kit 解析 QR；只回调一次并进入 CONNECTING | N/A |
| PERMISSION_DENIED | CAMERA 未授权或被系统撤销 | 不启动相机；保留中文授权说明与重新授权按钮 | 不显示黑屏相机层 |
| CAMERA_OPEN_FAILURE | Camera1 枚举/打开/设置预览失败 | 取景框显示明确中文失败提示，可返回上一步；不崩溃、不静默 | 释放已创建资源 |
| NO_QR_OR_INVALID_QR | 相机正常但画面无二维码/二维码 payload 无效 | 持续预览；无效 payload 沿用现有错误文案且不进入连接 | 预览继续可用，可重新扫描 |
| LEAVE_SCAN | 离开扫码页或进入 CONNECTING | 停止预览、移除回调、释放 Camera 与 Surface；不持有摄像头 | 防止隐私指示灯/相机资源残留 |

</frozen-after-approval>

## Code Map

- `companion-android/app/src/main/java/com/egosync/companion/pairing/QrScanner.kt` -- 相机权限门、Camera1 预览显示、预览帧转换与 ML Kit QR 分析的实现位置；当前含临时探针与旧 CameraX 路径。
- `companion-android/app/src/main/java/com/egosync/companion/pairing/PairingScreen.kt` -- 扫码步骤、取景框 overlay、诊断文字临时展示；需恢复正式 UI 并接入相机组件。
- `companion-android/app/src/main/java/com/egosync/companion/pairing/PairingViewModel.kt` -- `onQrScanned` 回调与 payload 校验/配对状态流，保持不变。
- `companion-android/app/src/main/AndroidManifest.xml` -- CAMERA 权限声明，保持不变。

## Tasks & Acceptance

**Execution:**
- [ ] `companion-android/app/src/main/java/com/egosync/companion/pairing/QrScanner.kt` -- 移除 CameraX 与临时 Camera1 探针，实现可显示的 Camera1 后摄预览，并将预览帧转换为 ML Kit `InputImage` 进行 QR 分析 -- 让卓易通环境实际显示画面并完成扫码。
- [ ] `companion-android/app/src/main/java/com/egosync/companion/pairing/PairingScreen.kt` -- 移除屏内诊断输出，恢复取景框内正式相机内容与现有 overlay -- 不改变既有配对页视觉与交互。
- [ ] `companion-android/app/src/test/java/com/egosync/companion/pairing/QrPayloadTest.kt` -- 回归 payload 校验契约，必要时补充 QR 回调只触发一次的可测试逻辑 -- 防止相机换装影响配对信任入口。

**Acceptance Criteria:**
- Given HarmonyOS 6.1 + 卓易通 1.0.10.75 且已授权相机，when 进入扫码页，then 取景框在 3 秒内显示后摄实时画面，不出现纯黑/纯灰占位。
- Given 取景框收到有效桌面二维码，when ML Kit 成功解析，then `onQrScanned` 只调用一次并进入既有 CONNECTING 流程。
- Given Camera1 打开或预览配置失败，when 扫码页可见，then 显示明确中文错误，不崩溃、不吞异常，并释放相机资源。
- Given 用户离开扫码页，when 进入欢迎页或连接页，then Camera1 预览停止、资源释放且后续页面不再占用相机。
- Given 标准 Android 真机执行同一路径，when 授权并进入扫码页，then 仍能显示预览并识别 QR。

## Design Notes

Camera1 的 NV21 预览帧与真实显示 Surface 是两条输出：显示使用 `SurfaceView`/`TextureView` 承载 `setPreviewDisplay` 或 `setPreviewTexture`，分析使用同一帧的 `InputImage.fromByteArray`（需正确传入预览宽高、NV21 格式与相机旋转角度）。不得用“无显示 SurfaceTexture”作为正式实现；取景框必须收到同一 Camera1 实例的可见预览。

Camera1 资源控制采用单一持有者：进入组合时创建并绑定，离开组合时先清除 callback、停止 preview、release camera，再释放显示 Surface。ML Kit 分析期间必须避免重复提交未完成帧；使用轻量锁/冷却保证单帧处理与一次性回调，不阻塞相机预览线程。

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL，产出 debug APK。
- `cd companion-android && ./gradlew :app:testDebugUnitTest` -- expected: BUILD SUCCESSFUL，既有 payload 与配对状态单测通过。

**Manual checks:**
- 在 HarmonyOS 6.1 + 卓易通 1.0.10.75 真机覆盖安装 APK：授权后取景框显示后摄画面；对准桌面 QR 后只进入一次连接中。
- 离开扫码页后重新打开其他相机应用：相机可正常使用，确认资源未泄漏。
