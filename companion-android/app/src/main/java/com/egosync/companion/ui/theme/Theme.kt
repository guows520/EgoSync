package com.egosync.companion.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable

private val DarkScheme = darkColorScheme(
    primary = BrandIndigoLight,
    onPrimary = DarkBackground,
    primaryContainer = BrandIndigo.copy(alpha = 0.25f),
    onPrimaryContainer = DarkTextPrimary,
    secondary = BrandAmberLight,
    onSecondary = DarkBackground,
    secondaryContainer = BrandAmber.copy(alpha = 0.22f),
    onSecondaryContainer = DarkTextPrimary,
    tertiary = BrandGreen,
    onTertiary = DarkBackground,
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
    primary = BrandIndigo,
    onPrimary = LightBackground,
    primaryContainer = BrandIndigo.copy(alpha = 0.14f),
    onPrimaryContainer = LightTextPrimary,
    secondary = BrandAmber,
    onSecondary = LightBackground,
    secondaryContainer = BrandAmber.copy(alpha = 0.14f),
    onSecondaryContainer = LightTextPrimary,
    tertiary = BrandEmerald,
    onTertiary = LightBackground,
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
        content = content,
    )
}
