package com.egosync.companion.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp

// M3 ColorScheme 对齐桌面语义（映射关系见 README 对照表）：
// 深色模式下主交互色沿用桌面语义 = indigo 实色 + 白字（桌面 dark:bg-indigo-600 按钮/气泡），
// 而非 M3 惯例的亮 tonal primary——设计语言以桌面母本为准。
private val DarkScheme = darkColorScheme(
    primary = BrandIndigo,
    onPrimary = DarkTextPrimary,
    primaryContainer = BrandIndigo.copy(alpha = 0.25f),
    onPrimaryContainer = DarkTextPrimary,
    secondary = BrandAmber,
    onSecondary = DarkTextPrimary,
    secondaryContainer = BrandAmber.copy(alpha = 0.22f),
    onSecondaryContainer = DarkTextPrimary,
    tertiary = BrandGreen,
    onTertiary = DarkTextPrimary,
    background = DarkBackground,
    onBackground = DarkTextPrimary,
    surface = DarkSurface,
    onSurface = DarkTextPrimary,
    surfaceVariant = DarkSurfaceElevated,
    onSurfaceVariant = DarkTextSecondary,
    outline = DarkOutline,
    outlineVariant = DarkOutline.copy(alpha = 0.6f),
    error = BrandError,
)

private val LightScheme = lightColorScheme(
    primary = BrandIndigoDeep,
    onPrimary = LightSurface,
    primaryContainer = BrandIndigoDeep.copy(alpha = 0.14f),
    onPrimaryContainer = LightTextPrimary,
    secondary = BrandAmber,
    onSecondary = LightSurface,
    secondaryContainer = BrandAmber.copy(alpha = 0.14f),
    onSecondaryContainer = LightTextPrimary,
    tertiary = BrandEmerald,
    onTertiary = LightSurface,
    background = LightBackground,
    onBackground = LightTextPrimary,
    surface = LightSurface,
    onSurface = LightTextPrimary,
    surfaceVariant = LightSurfaceElevated,
    onSurfaceVariant = LightTextSecondary,
    outline = LightOutline,
    outlineVariant = LightOutline.copy(alpha = 0.7f),
    error = BrandError,
)

/**
 * 圆角 Token — 母本：index.css --radius-button 6px / --radius-card 10px /
 * --radius-dialog 12px / --radius-input 24px（胶囊）。
 */
private val EgoSyncShapes = Shapes(
    extraSmall = RoundedCornerShape(6.dp),   // --radius-button
    small = RoundedCornerShape(6.dp),        // --radius-button
    medium = RoundedCornerShape(10.dp),      // --radius-card
    large = RoundedCornerShape(12.dp),       // --radius-dialog
    extraLarge = RoundedCornerShape(24.dp),  // --radius-input（胶囊形态）
)

/** 应用主题模式：dark 为默认。 */
enum class ThemeMode { DARK, LIGHT }

@Composable
fun EgoSyncTheme(
    themeMode: ThemeMode = ThemeMode.DARK,
    content: @Composable () -> Unit,
) {
    MaterialTheme(
        colorScheme = when (themeMode) {
            ThemeMode.DARK -> DarkScheme
            ThemeMode.LIGHT -> LightScheme
        },
        typography = EgoSyncTypography,
        shapes = EgoSyncShapes,
        content = content,
    )
}
