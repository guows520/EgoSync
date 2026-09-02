# Investigation: 安卓配对扫码取景框全黑

## Hand-off Brief

1. **What happened.** 用户报告：单击「开始配对」并授权相机后，取景框全黑（推测"摄像头没打开"——此为用户假设，未经证据确认）。
2. **Where the case stands.** 静态代码审查完成：扫码组件链路（Manifest 权限→权限门→CameraX 绑定→PreviewView）接线正确，但 `bindToLifecycle` 的绑定异常被 `catch (_: Exception)` 静默吞掉——任何绑定失败都会精确呈现为"全黑取景框、零提示"，与症状完全吻合。缺少设备端日志，无法在「相机未打开」与「已打开但预览未渲染」之间裁决。
3. **What's needed next.** 先做零成本验证：授权后观察相机隐私绿点（Android 12+）+ `adb logcat -s CameraX` 抓一次复现；若仍不能裁决，在 QrScanner 绑定链路加诊断日志（见 Recommended Next Steps）。

## Case Info

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| Ticket           | N/A（用户口述）                                                            |
| Date opened      | 2026-05-28                                                                 |
| Status           | Active                                                                     |
| System           | companion-android（Compose + CameraX 1.6.2 + ML Kit；设备/系统版本未知）   |
| Evidence sources | 源码（QrScanner.kt / PairingScreen.kt / PairingViewModel.kt / AppNavHost.kt / AndroidManifest.xml / build.gradle.kts）、git log |

## Problem Statement

安卓版单击「开始配对」（WELCOME→SCAN），系统权限弹窗出现并授权后，扫码取景框呈全黑。用户假设：摄像头没有打开。

## Evidence Inventory

| Source                | Status    | Notes                                                        |
| --------------------- | --------- | ------------------------------------------------------------ |
| 源码（配对扫码链路）  | Available | 全链路已通读，见 Confirmed Findings                          |
| 设备 logcat           | Missing   | 区分「绑定失败」vs「预览渲染失败」的关键证据，未采集          |
| 测试环境信息          | Missing   | 真机型号/Android 版本 vs 模拟器（AVD 相机配置）未知           |
| 相机隐私指示灯观察    | Missing   | Android 12+ 绿点直接回答"摄像头是否在用"，用户未报告          |
| `dumpsys media.camera`| Missing   | 可查相机设备被哪个进程占用/是否打开                          |

## Investigation Backlog

| #  | Path to Explore                                  | Priority | Status | Notes                              |
| -- | ------------------------------------------------ | -------- | ------ | ---------------------------------- |
| 1  | logcat 抓取 CameraX/Camera2 绑定结果             | High     | Open   | 一条命令，裁决 H1 vs H3            |
| 2  | 确认测试环境（真机/模拟器 + AVD 相机配置）       | High     | Open   | 模拟器 camera=none 时必然黑屏      |
| 3  | 隐私绿点观察                                     | High     | Open   | 零成本，直接验证用户假设           |
| 4  | QrScanner 绑定链路诊断日志（需用户确认后实施）   | Medium   | Open   | 规则十三：无法裁决时的手段         |
| 5  | PreviewView implementationMode 兼容性实验        | Low      | Open   | 仅当 H1 被排除后启用               |

## Timeline of Events

| Time        | Event                                             | Source                  | Confidence |
| ----------- | ------------------------------------------------- | ----------------------- | ---------- |
| 用户操作    | 点「开始配对」→ WELCOME→SCAN（`startScan`）        | PairingViewModel.kt:45  | Confirmed  |
| 系统弹窗    | `LaunchedEffect` 触发 CAMERA 权限请求              | QrScanner.kt:173-175    | Confirmed  |
| 授权回调    | `granted=true` → 重组渲染 QrScannerView            | QrScanner.kt:169-171    | Confirmed  |
| 此后        | factory 执行 → ProcessCameraProvider 初始化 → bind | QrScanner.kt:62-101     | Confirmed（代码路径）；绑定成败未知 |
| 观测        | 取景框全黑                                        | 用户口述                | 用户假设为"相机没开"，未验证 |

## Confirmed Findings

### Finding 1: 相机绑定异常被静默吞掉，无日志、无 UI 反馈

**Evidence:** `companion-android/app/src/main/java/com/egosync/companion/pairing/QrScanner.kt:85-95`

```kotlin
try {
    provider.unbindAll()
    provider.bindToLifecycle(...)
} catch (_: Exception) {
    // 相机被占用/不可用：取景框保持可见，用户可返回或重试
}
```

**Detail:** `bindToLifecycle` 抛出的任何异常（相机被其他进程占用、HAL 不可用、UseCase 冲突等）都被丢弃。绑定失败时 Preview 无画面，UI 无任何错误呈现。注释里的"用户可返回或重试"在代码中并无对应出口——界面上没有重试按钮。这同时违反项目规则十二（显式失败）。

### Finding 2: 取景框底色 = 0.55 透明黑叠加深色背景，预览缺失时呈现"全黑"

**Evidence:** `PairingScreen.kt:160-166` — `Box(... .background(Color.Black.copy(alpha = 0.55f)))`，外层 `Surface(color = background)` 深色主题下为 `#0F1117`（README 色彩表）。

**Detail:** 预览未渲染时用户看到的就是近纯黑的取景框——症状与"预览缺失"完全一致，与"相机没打开"（预览缺失的一个子集）也一致。

### Finding 3: 扫码组件接线本身正确（权限→门→绑定→预览）

**Evidence:**
- Manifest 已声明 CAMERA：`AndroidManifest.xml:6`
- CameraX 四件套 + ML Kit 依赖齐全：`build.gradle.kts:75-79`
- 权限门逻辑正确（含 ON_RESUME 自动解除拒绝态）：`QrScanner.kt:159-217`
- `preview.setSurfaceProvider(previewView.surfaceProvider)` 已接线：`QrScanner.kt:73-75`
- 离开组合即解绑（P4 资源纪律）：`QrScanner.kt:102-105`

**Detail:** 静态审查未发现结构性接错；问题在运行时行为，无法从代码单独定论。

### Finding 4: ProcessCameraProvider 初始化失败不会静默——会直接崩

**Evidence:** `QrScanner.kt:71` — `providerFuture.get()` 位于 try 块**之外**。

**Detail:** 若 provider 初始化失败，异常会抛在主线程 listener 里导致 crash，而非黑屏。用户未报告 crash → provider 初始化成功，问题（若存在）出在 bind 或渲染阶段。此发现收窄了假设空间。

### Finding 5: ML Kit 分析器堵帧不会导致预览黑屏

**Evidence:** `QrScanner.kt:76-84` — ImageAnalysis 是独立 use case；`STRATEGY_KEEP_ONLY_LATEST` 下 analyzer 不关帧只丢分析帧，Preview 通路不受影响。

**Detail:** 可基本排除"分析器堵帧→黑屏"路径（注：`analyze` 中 `process()` 若抛异常无 `addOnFailureListener`，帧会不关闭，但同样只影响分析不影响预览）。

## Deduced Conclusions

### Deduction 1: 全黑 = Preview 未渲染画面

**Based on:** Finding 2 + Finding 3

**Reasoning:** 组件链路接线正确 → 若绑定成功且渲染正常，取景框应显示相机画面；观察到全黑 → Preview 通路中断。中断点只可能在：① bind 失败（Finding 1 的静默 catch）或 ② bind 成功但 PreviewView 不渲染。

**Conclusion:** "摄像头没打开"（用户假设）只是中断点 ① 的一个子情况（如相机被占用/不可用）；不能确认。需要设备证据裁决。

## Hypothesized Paths

### Hypothesis 1: bindToLifecycle 抛异常被吞（相机未打开或绑定失败）

**Status:** Open（最强嫌疑）

**Theory:** 相机被占用（其他 App/进程持有）、设备相机不可用、或 CameraX 与该设备 HAL 不兼容 → bind 抛异常 → 静默吞掉 → 黑屏。

**Supporting indicators:** Finding 1（catch 吞异常与症状精确吻合）；注释自己列出的触发场景就是"相机被占用/不可用"。

**Would confirm:** logcat 复现时出现 `CameraX`/`Camera2` 错误（如 `UseCaseConflictException`、`CameraUnavailableException`、`IllegalArgumentException`）；相机隐私绿点不亮；`dumpsys media.camera` 显示无客户端打开。

**Would refute:** 绿点亮 + logcat 显示绑定成功 + PreviewView state 为 `STREAMING`。

**Resolution:** 待设备证据。

### Hypothesis 2: 测试环境为模拟器且 AVD 相机配置不可用

**Status:** Open

**Theory:** AVD 的 back camera 配置为 `none`（或虚拟设备相机初始化失败）→ DEFAULT_BACK_CAMERA 无可用设备 → bind 抛异常 → 同 H1 黑屏。

**Supporting indicators:** 用户未说明测试环境；模拟器测试在该项目中是常规操作（README 构建/测试均在开发机）。

**Would confirm:** AVD `config.ini` 中 `hw.camera.back = none`；改 AVD 相机配置为 `virtualscene`/`emulated` 后预览出现。

**Would refute:** 真机复现。

**Resolution:** 待环境信息。

### Hypothesis 3: PreviewView（PERFORMANCE 模式 SurfaceView）在 Compose 层级下渲染失败

**Status:** Open（较低嫌疑）

**Theory:** PreviewView 默认 ImplementationMode=PERFORMANCE 使用 SurfaceView；Compose 的 `AndroidView` + 父级 `.clip(RoundedCornerShape)`（`PairingScreen.kt:164`）组合下，SurfaceView 的 hole-punching/定位更新可能失效 → 绑定成功但画面黑。

**Supporting indicators:** Compose + SurfaceView 组合存在已知渲染边缘案例。

**Would confirm:** logcat 无绑定错误、绿点亮，但画面黑；改 `previewView.implementationMode = ImplementationMode.COMPATIBLE`（TextureView）后画面出现。

**Would refute:** logcat 显示绑定失败（落入 H1/H2）。

**Resolution:** 待排除 H1/H2 后实验。

### Hypothesis 4: 权限授予时序导致 bind 早于 lifecycle STARTED

**Status:** Open（低嫌疑——bindToLifecycle 会自动等待 STARTED，预期表现为延迟显示而非永久黑）

**Resolution:** 若 H1-H3 全被排除再查。

### 已登记的用户原始假设（Hypothesis 0）：摄像头没有打开

**Status:** Open（= H1/H2 的通俗表述，尚未验证）
**Resolution:** 隐私绿点 / logcat 可直接验证。

## Missing Evidence

| Gap                      | Impact                                    | How to Obtain                              |
| ------------------------ | ----------------------------------------- | ------------------------------------------ |
| 复现时的 logcat          | 直接裁决 bind 成败（H1/H2 vs H3）         | `adb logcat -s CameraX Camera2 camera HAL` |
| 测试环境（真机/模拟器）  | H2 成立与否；决定后续路径                 | 询问用户 / 查看设备                        |
| 相机隐私绿点             | 直接回答"摄像头开了没"（用户原始疑问）     | 授权后肉眼观察状态栏                       |
| `dumpsys media.camera`   | 相机是否被其他进程占用                    | `adb shell dumpsys media.camera`           |

## Source Code Trace

| Element       | Detail                                                                 |
| ------------- | ---------------------------------------------------------------------- |
| Error origin  | `QrScanner.kt:93` `catch (_: Exception)`（异常吞没点）；症状呈现于 `PairingScreen.kt:160-166` 取景框 |
| Trigger       | WELCOME→SCAN 后授权相机 → `QrScannerView` factory → `bindToLifecycle`   |
| Condition     | 绑定抛异常（占用/不可用）或预览渲染失败，均无反馈 → 全黑               |
| Related files | `PairingViewModel.kt`（步骤状态机）、`AppNavHost.kt:160-195`（配对路由）、`AndroidManifest.xml:6`、`build.gradle.kts:75-79` |

## Conclusion

**Confidence:** Medium

代码接线正确；确定的是：**绑定失败会被静默吞掉且 UI 呈现恰好为全黑取景框**（Confirmed），即症状与该静默失败路径精确吻合。未确定的是黑屏时相机绑定究竟成功与否（缺失设备日志）。最强假设 H1/H2（相机未打开/不可用，含模拟器无相机），次强 H3（SurfaceView 渲染）。用户的原始假设"没有打开摄像头"成立与否，用隐私绿点 + logcat 一步即可验证。

## Recommended Next Steps

### Diagnostic（按成本从零到有排序）

1. **零成本观察**：授权后看状态栏相机绿点（Android 12+）。绿点亮 = 摄像头已打开，问题在渲染（H3）；不亮 = 相机确实没打开（H1/H2）。
2. **logcat 抓取**：复现时执行 `adb logcat -s CameraX Camera2 CameraDeviceClient HAL`。有绑定错误堆栈 → H1/H2 实锤。
3. **环境确认**：真机还是模拟器？模拟器则检查 AVD 相机配置（back camera 是否为 none）。
4. **诊断日志（需用户确认后实施，规则十三）**——若 1-3 仍无法裁决，在以下位置加 `CompanionLog`：
   - `QrScanner.kt:69` providerFuture listener 进入处：`"QrScan: providerFuture 完成"`；
   - `QrScanner.kt:71` `get()` 后：`"QrScan: provider 就绪"`；
   - `QrScanner.kt:92` bind 成功后：`"QrScan: 相机绑定成功"`；
   - `QrScanner.kt:93` catch 内：`"QrScan: 相机绑定失败: ${e.javaClass.simpleName}: ${e.message}"`（含异常类型，不涉 QR 内容）；
   - `QrScanner.kt:104` onDispose：`"QrScan: 相机解绑"`；
   - `CameraPermissionGate` granted 翻转处（`QrScanner.kt:171/182`）：`"QrScan: 权限状态 granted=$granted"`。
   - 定位后全部移除。

### Fix direction（根因确认后）

- 无论哪个假设成立，**Finding 1 的静默吞异常都应修复**：catch 内记录日志并向 UI 呈现失败态（项目规则十二：显式失败）。这是本症状"无从定位"的直接原因。
- 若为 H2：测试环境侧改 AVD 相机配置，无代码改动。
- 若为 H3：PreviewView 设 `ImplementationMode.COMPATIBLE`。

## Reproduction Plan

真机/模拟器安装 debug APK → 首跑进入配对流 → 点「开始配对」→ 授权相机 → 观察取景框（预期黑屏）→ 同步抓 logcat 与绿点状态。

## Follow-up: 2026-05-28

### New Evidence

- 用户确认测试环境为**真机**（非模拟器）→ H2 Refuted。
- 用户报告授权后黑屏期间**未见相机隐私绿点**。前提条件（Android ≥ 12）尚未确认——若设备为 Android 11-，绿点本就不存在，该证据无效。

### Additional Findings

- 绿点观察（若设备 Android ≥ 12 成立）指向：摄像头未被任何进程打开 → H1（绑定失败被吞）强烈趋近 Confirmed，H3（渲染失败）趋近 Refuted（渲染失败时相机是打开的，绿点应亮）。

### Updated Hypotheses

| # | 状态变化 | 说明 |
|---|---------|------|
| H0 摄像头没打开 | 趋近 Confirmed（待 Android 版本确认） | 绿点不亮 + 真机 |
| H1 绑定异常被吞 | 趋近 Confirmed | 同上 |
| H2 模拟器无相机 | **Refuted** | 真机 |
| H3 SurfaceView 渲染 | 趋近 Refuted | 渲染失败时绿点应亮 |

### Backlog Changes

- #2（环境确认）→ Done。#3（绿点观察）→ Done（结果：不亮，待版本前提确认）。

### Updated Conclusion

**Confidence: Medium → Medium-High（待 Android 版本与 logcat 最终确认）。**
真机 + 绿点不亮（若 Android ≥ 12）→ 相机确实没打开，最可能路径：`bindToLifecycle` 抛异常被 `QrScanner.kt:93` 静默吞掉。仍需 logcat 给出异常类型，并确认 Android 版本以使绿点证据生效。

## Follow-up: 2026-05-28 #2

### New Evidence

- 用户手机为**鸿蒙（HarmonyOS）**，APK 经**卓易通**（纯血鸿蒙上的 Android 兼容层）安装运行。
- web 检索：卓易通生态相机类兼容问题普遍——拍照存系统相册失败（bbs.itying.com/topic/6963cb2c6f4d61004c8394a8）、uni-app 拍照异常（68286253062dc60098c27d82）、系统升级后安卓应用全部失效（69357ba42fcc650041be93c5）。无 CameraX 黑屏的直接公开报告。

### Additional Findings

- **环境事实重构**：app 并非跑在原生 Android 上，而是卓易通的 Android 兼容容器内。CameraX → Camera2 → 卓易通容器 → 鸿蒙相机服务，任何一层转译缺陷都会表现为绑定失败（且被 `QrScanner.kt:93` 吞掉）→ 黑屏。
- **绿点证据降级**：鸿蒙的摄像头隐私指示机制与 Android 12 绿点不同，且兼容层容器内的相机访问是否触发系统隐私指示未确认。"没看到绿点"不再能强证明"相机没打开"。

### Updated Hypotheses

| # | 状态变化 | 说明 |
|---|---------|------|
| H5（新）卓易通兼容层相机链路缺陷 | **Open（最强嫌疑）** | 容器对 Camera2/CameraX 支持不完整，bind 失败被吞 → 黑屏 |
| H1 绑定异常被吞 | 与 H5 合并考虑 | 吞异常是确定的放大器；异常的**来源**现在更可能是卓易通层 |
| H0 用户原始假设"相机没开" | 仍 Open | 在 H5 下相机可能确实没开（容器未能打开） |
| H3 SurfaceView 渲染 | Open（降） | 仍可能，但需先过兼容层这关 |

### Backlog Changes

- 新增：卓易通环境分叉测试（其他安卓 app 相机是否正常）→ Open（High）
- 新增：确认 HarmonyOS 版本与卓易通版本 → Open（High）
- #1 logcat：卓易通容器下 adb/logcat 可用性未知 → 待验证

### Updated Conclusion

**Confidence: Medium（假设面已重构）。** 环境=卓易通兼容层是新的主导变量：CameraX 黑屏很可能并非本 app 代码缺陷，而是卓易通对相机 API 栈的兼容性限制；`QrScanner.kt:93` 的静默吞异常则剥夺了本应可见的错误信息。待分叉测试裁决：同一环境下其他安卓 app 的相机是否正常。

## Follow-up: 2026-05-28 #3

### New Evidence

- 分叉测试结果：同一卓易通环境下**其他安卓 app 相机正常**。
- 环境：HarmonyOS 6.1.0 + 卓易通 1.0.10.75。
- 用户决策：**卓易通属于目标运行环境**（其所有安卓应用都正常使用）。

### Additional Findings

- H5 全局性兼容缺陷被**收窄**：卓易通相机通路可用，但"其他 app 正常"不能等同"CameraX 正常"——多数主流 app 走 Camera1/自研相机栈，而本 app 走 CameraX `bindToLifecycle` + PreviewView（SurfaceView）。兼容层对这条新路径的支持仍是首要嫌疑（记为 H5'：**卓易通对 CameraX/Camera2 新路径支持不完整**）。
- 项目决策影响：卓易列入支持环境后，本案为真实待修 bug，非环境限制。

### Updated Hypotheses

| # | 状态 | 说明 |
|---|------|------|
| H5' 卓易通对 CameraX 路径支持不完整 | Open（最强） | bind 失败（如 DEFAULT_BACK_CAMERA 在容器内映射失败）→ 被吞 → 黑屏 |
| H3 SurfaceView 在容器内渲染失败 | Open（次强，回升） | 其他 app 可能用 TextureView/Camera1+SurfaceHolder 不同路径；容器内 SurfaceView hole-punch 失效会精确呈现黑屏且不报错 |
| H0 用户假设"相机没开" | 仍 Open | 若 H5' 成立则成立 |
| H2 模拟器 | Refuted（上一轮） | |

### Missing Evidence（更新）

| Gap | 影响 | 获取方式 |
| --- | --- | --- |
| bind 成败 + 异常类型 | 裁决 H5' | 见诊断方案 |
| Preview 流状态（STREAMING/IDLE） | 裁决 H3 | PreviewView previewStreamState |
| 卓易通环境下 logcat 可达性 | 未知 | 诊断以**屏幕内呈现**为最可靠通道 |

### Backlog Changes

- 分叉测试 → Done（其他 app 正常）。
- 新增（High）：临时诊断方案实施——绑定成败/异常类型/预览流状态**直接显示在扫码页 UI 上**（logcat 在卓易通环境可达性未知，屏幕是最可靠证据通道）。待用户确认后实施（规则十三）。

### 诊断实施记录（2026-05-28 #3，用户已确认方案 A）

临时代码统一标记 `INVESTIGATE-TMP`，定位后按标记整体移除：

| 位置 | 诊断内容 |
| --- | --- |
| `QrScanner.kt` QrScannerView 签名 | 新增 `onDiagnostic: (String) -> Unit = {}` 回调（默认空实现，不影响其他调用点） |
| `QrScanner.kt` factory 内 | `previewStreamState` 观察：报告 `预览流状态: IDLE/STREAMING`（裁决 H3） |
| `QrScanner.kt` providerFuture listener | `provider 就绪, 相机数: N`（容器内相机枚举） |
| `QrScanner.kt` bind 成功 | `相机绑定成功` |
| `QrScanner.kt` catch | `相机绑定失败: {异常类名}: {消息}`（原静默吞异常点，裁决 H5'） |
| `PairingScreen.kt` ScanStep | `mutableStateListOf` 收集 + 取景框下方红色 labelSmall 小字展示最近 6 条（去重追加） |

### Updated Conclusion

**Confidence: Medium。** 假设收敛为两条：H5'（CameraX 绑定在卓易通失败被吞）与 H3（SurfaceView 渲染在卓易通失效）。`QrScanner.kt:93` 静默吞异常使两者在用户侧表现相同（黑屏无反馈）。裁决手段：临时诊断显示绑定结果 + 异常类型 + 预览流状态，定位后移除。

## Follow-up: 2026-05-28 #4

### New Evidence（诊断版 APK 屏内回传，截图）

```text
[诊断] provider 就绪, 相机数: 7
[诊断] 相机绑定成功
[诊断] 预览流状态: IDLE        ← 始终未变 STREAMING（观察 ~10s）
```

### Additional Findings

- **H5'（bind 失败）Refuted**：provider 初始化成功、容器枚举 7 个相机、`bindToLifecycle` 未抛异常。
- **H3 确认到细分机制**：绑定成功但预览流恒为 IDLE——PreviewView（默认 PERFORMANCE 模式 = **SurfaceView**）在卓易通容器内拿不到帧。相机 open 了，但 capture session 的输出没有进入 SurfaceView 的 surface → 黑屏。
- 该机制同时解释"其他 app 正常"：多数 app 用 TextureView/Camera1+SurfaceHolder 等不同路径，绕开了容器对 SurfaceView hole-punch 的兼容缺陷。
- ML Kit 分析器大概率同样收不到帧（ImageAnalysis 与 Preview 一样无帧可分析）——扫码功能整体失效，与症状一致。

### Updated Hypotheses

| # | 状态 | 说明 |
|---|------|------|
| H3'（H3 细分）SurfaceView 渲染路径在卓易通容器内失效 | **Confirmed（机制层面）** | 绑定成功 + 流恒 IDLE，与 SurfaceView surface 未生效的预期完全一致 |
| H5' bind 失败 | Refuted | 绑定成功是截图直接证据 |
| H0"相机没开" | 精确化 | 相机进程层面打开了（bind 成功），但画面通路断在 SurfaceView → 用户视角"没开" |

### Updated Conclusion

**Confidence: High（机制）/ 待验证（修复）**。根因：卓易通容器内 PreviewView 默认的 SurfaceView（PERFORMANCE 模式）渲染路径拿不到相机帧，预览流恒 IDLE → 取景框全黑；`QrScanner.kt` 的静默 catch 在本案未触发，但仍是同类故障零反馈的结构性风险。修复假设：切换 `ImplementationMode.COMPATIBLE`（TextureView，走常规 View 渲染管线，不依赖 hole-punch）→ 预期流转 STREAMING、画面出现。

### Backlog Changes

- 验证性修复实验：PreviewView 设 COMPATIBLE（临时带 INVESTIGATE-TMP 标记），重建 APK → 用户复测。通过后：移除全部 INVESTIGATE-TMP 诊断代码，COMPATIBLE 一行转为正式修复（附注释说明卓易通适配依据）。

## Follow-up: 2026-05-28 #5

### New Evidence

- COMPATIBLE（TextureView）版复测：诊断输出**不变**——`provider 就绪, 相机数: 7` / `相机绑定成功` / `预览流状态: IDLE`（无 STREAMING）。

### Additional Findings

- **结论修正**：`bindToLifecycle` 返回成功只代表 use case 注册完成；相机 open 是其后的**异步**动作，open 失败不会抛异常，而是走 `CameraInfo.cameraState` 错误通道（未被监听）。"相机确实打开了"此前判断过早。
- H3'（SurfaceView 特有缺陷）**Refuted**：TextureView 同样拿不到帧 → 问题不在 hole-punch 渲染层，而在更底层：相机异步 open 失败或 capture session 未启动。
- 容器枚举 7 个相机但可能全部/部分是映射到死设备的虚拟相机——DEFAULT_BACK_CAMERA 选中的具体设备存疑。

### Updated Hypotheses

| # | 状态 | 说明 |
|---|------|------|
| H6（新）相机异步 open 失败 / session 未启动，错误落在未监听的 CameraState 通道 | Open（最强） | bind 成功 + 两种渲染模式均 IDLE 的唯一自洽解释 |
| H3' SurfaceView 缺陷 | Refuted | TextureView 同样失败 |

### Backlog Changes

- 诊断第二轮（延续已确认的方案 A）：监听 `cameraInfo.cameraState`（状态+错误码）+ 分析器帧计数（判定是否有任何 use case 收到帧）+ lensFacing 报告。

## Follow-up: 2026-05-28 #6

### New Evidence（第三轮诊断 APK 屏内回传，截图）

```text
[诊断] 相机绑定成功
[诊断] 选中相机 lensFacing: 1          ← 1=FRONT；请求的是 BACK(0)
[诊断] 相机状态: CLOSED
[诊断] 相机状态: OPENING
[诊断] 相机状态: CLOSING               ← 未经过 OPEN
[诊断] 相机状态: CLOSED
（无 分析帧数 行——分析器零帧；无错误类型——cameraState.error 为空）
```

### Additional Findings

- **H6 Confirmed（机制）**：相机异步 open 发起后从未完成——OPENING 直接转 CLOSING→CLOSED，无 OPEN、无错误回调（CameraState.error 为空）。这是容器相机服务静默失败的特征签名：open 请求发出，HAL/容器侧无响应或被无声放弃。
- **容器相机元数据自相矛盾**：`requireLensFacing(BACK=0)` 绑定成功，返回的 CameraInfo 却报 `lensFacing=1(FRONT)`——容器的相机枚举元数据不可信。
- 应用侧 CameraX 代码行为完全正常：状态机转换、绑定、observer 全部按规范工作；分析器零帧与预览零帧一致（相机根本没出帧）。
- 黑屏 = 相机从未产出任何帧 + PreviewView 无内容，与渲染模式无关（SurfaceView/TextureView 双双排除）。

### Updated Hypotheses

| # | 状态 | 说明 |
|---|------|------|
| H6 卓易通容器相机 open 静默失败（无错误回传）+ 元数据不一致 | **Confirmed** | 三轮屏内证据闭环：绑定成功→open 发起→永不 OPEN→无错误→零帧 |
| H3' 渲染层（Surface/TextureView）| Refuted | 两种模式同样零帧，相机侧即断流 |
| H5' bind 失败 | Refuted | 绑定成功 |

### Updated Conclusion

**Confidence: High。** 根因：卓易通 1.0.10.75 容器的 Camera2 模拟层对 CameraX 的相机 open 静默失败（OPENING 后无 OPEN、无错误回调、直接 CLOSED），且相机 facing 元数据与请求不符。这不是本 app 代码缺陷——应用侧一切按规范工作，被容器层吞掉的是 open 本身。剩余待裁决：容器是否对**任何**相机/任何 API（如 Camera1）能成功出帧——决定应用侧是否存在 workaround。

### Backlog Changes

- 第四轮实验（低成本裁决）：改绑 DEFAULT_FRONT_CAMERA——若前摄能出帧 → 容器仅个别相机映射坏（可探索选相机 workaround）；若同样 OPENING→CLOSED → 容器相机 open 整体不可用（CameraX 路径无解，转 Camera1 探针或接受限制）。

## Follow-up: 2026-05-28 #7

### New Evidence

- 用户否决前摄实验：其结果不改变产品决策，成本（安装+复测）不值——正确。
- 决策修正：跳过前摄实验，直接测**决定性变量**——Camera1（`android.hardware.Camera`）在容器内能否出帧。理由：容器里其他 app 相机正常，多为 Camera1/老路径；Camera1 可用则扫码组件存在真实修复路径（Camera1+ML Kit 重写），不可用则应用侧无解。一轮构建定案。

### Backlog Changes

- 前摄实验 → Cancelled（用户裁决，成本不值）。
- 第四轮改为：Camera1 最小探针（不渲染、仅枚举相机/open/取帧计数），整体替代 CameraX 路径注入 QrScannerView；扫描 UI/权限门不动。

## Side Findings

- `README.md` 仍描述扫码步为「扫码取景模拟…点『模拟扫码成功』推进」——与已换装的真实扫码实现（Story 12.4，commit a1dc62a）不符，文档过时（Confirmed，README「页面地图」节）。
- `QrScanner.kt:94` 注释声称"用户可返回或重试"，但黑屏态下取景框内无重试入口，仅有「返回上一步」——注释与实际能力不符（Confirmed）。
