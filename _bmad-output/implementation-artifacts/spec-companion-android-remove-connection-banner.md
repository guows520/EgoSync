---
title: 'companion-android 移除三态连接横幅'
type: 'chore'
created: '2026-05-27'
status: 'done'
route: 'one-shot'
---

# companion-android 移除三态连接横幅

## Intent

**Problem:** 原型 mock 恒为 Direct 态，FR-40 三态横幅变成常驻绿色块，占据顶部空间且无信息量；用户裁决整个横幅组件去掉。离线明示由 DegradedOverlay 独立承担，「我的」页配对设备卡保留连接状态观察点，删除无功能损失。

**Approach:** 删除 ConnectionStatusBanner.kt 组件（含 Preview）+ AppNavHost 去 topBar 挂载 + 全仓清理过时「横幅」文案（README 5 处、代码注释 3 处）。

## Suggested Review Order

1. [挂载点移除](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt) — MainShellRoute 的 Scaffold 去 topBar 与死变量 connectionState，函数头注释同步
2. [组件删除](../../companion-android/app/src/main/java/com/egosync/companion/connection/ConnectionClient.kt) — connection/ 目录现仅剩状态机与客户端（横幅文件已删）
3. [用户可见文案](../../companion-android/app/src/main/java/com/egosync/companion/ui/settings/SettingsScreen.kt) — Debug 状态模拟说明改为「驱动降级态遮罩与操作禁用态」
4. [文档同步](../../companion-android/README.md) — 页面地图/全局组件/Debug 表/替换点说明/目录地图五处去横幅描述

## Verification

**Commands:**
- `./gradlew :app:testDebugUnitTest` -- expected: BUILD SUCCESSFUL（编译零残留引用 + 全部单测通过）

**Manual checks (if no CLI):**
- 设备运行：顶部不再有绿色横幅；状态栏正常（Scaffold 无 topBar 时 contentPadding 自动取系统栏 inset）；「我的」页配对设备卡仍显示连接状态圆点
