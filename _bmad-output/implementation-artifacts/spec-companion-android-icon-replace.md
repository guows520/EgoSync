---
title: 'companion-android 启动图标替换为桌面版'
type: 'chore'
created: '2026-05-27'
status: 'done'
route: 'one-shot'
---

# companion-android 启动图标替换为桌面版

## Intent

**Problem:** 移动端用自制"同心圆环"矢量图作启动图标，与桌面端 indigo 品牌图标视觉不一致；桌面 `src-tauri/icons/android/` 已有全套 mipmap 资源未被使用。

**Approach:** 搬运桌面 adaptive icon 全套资源（foreground PNG × 5 密度 + adaptive XML + 白底背景色），删除自制矢量图；对 Tauri 生成器产出做安全区后处理（foreground 内容缩放居中至 66dp 安全区），修复 hdpi legacy PNG 尺寸异常并清理 round 死资源。

## Suggested Review Order

1. [adaptive icon 定义](../../companion-android/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml) — 桌面同源：background 取 color、foreground 取 mipmap
2. [背景色](../../companion-android/app/src/main/res/values/colors.xml) — ic_launcher_background 改 #fff（与桌面 values 一致），app_background 主题色未动
3. [foreground 安全区后处理](../../companion-android/app/src/main/res/mipmap-xxxhdpi/ic_launcher_foreground.png) — 5 密度内容均收敛至中心 66dp（原 Tauri 产出 94×88dp 超出，会被 launcher 裁切）
4. [hdpi 修复](../../companion-android/app/src/main/res/mipmap-hdpi/ic_launcher.png) — 桌面产物 49×49 异常，从 xxxhdpi 192px 下采样为规范 72×72

## Verification

**Commands:**
- `./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL（16 个图标资源链接打包通过）
- `python3 -c "from PIL import Image; ..."` -- expected: foreground 各密度非透明 bbox ≤ 66dp 安全区像素；hdpi ic_launcher 为 72×72

**Manual checks (if no CLI):**
- 设备安装后 launcher 图标为桌面版 indigo logo、白底、圆形/圆角遮罩下外环完整不被裁切
