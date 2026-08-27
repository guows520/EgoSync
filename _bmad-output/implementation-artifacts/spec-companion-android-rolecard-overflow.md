---
title: 'companion-android 角色卡纵向溢出修复'
type: 'bugfix'
created: '2026-05-27'
status: 'done'
route: 'one-shot'
---

# companion-android 角色卡纵向溢出修复

## Intent

**Problem:** 仪表盘底部角色卡固有高度约 267dp，超过 pager 页可用高度（360×800dp 屏约 238dp），统计行"任务/记忆/会话/待办"标签在页底缘被裁（用户实测症状：标签下半部分被遮盖）。

**Approach:** 双保险：压缩卡内边距与间距（padding 18→12、Spacer 18/20/14→12/12/10，省约 30dp ≥ 29dp 溢出）使常规屏首屏完整显示；pager 页内加 verticalScroll 作极矮屏兜底（横向翻页与纵向滚动正交，每页滚动位置独立保存）。

## Suggested Review Order

1. [pager 页内滚动兜底](../../companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt) — HorizontalPager page lambda 内 Box(fillMaxSize).verticalScroll 包裹角色卡
2. [卡内间距压缩](../../companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt) — Column padding 与三处 Spacer 收紧，注释说明压缩量与依据

## Verification

**Commands:**
- `./gradlew :app:testDebugUnitTest` -- expected: BUILD SUCCESSFUL

**Manual checks (if no CLI):**
- 360×800dp 模拟器：仪表盘角色卡首屏完整显示"任务/记忆/会话/待办"统计行与"查看记忆"按钮，无裁切；横向翻页正常
- 更矮屏幕（如 320×568dp）：卡片可纵向滚动至底
