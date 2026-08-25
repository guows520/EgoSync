package com.egosync.companion.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

// 系统默认字体栈（不引入在线字体）
private val Base = TextStyle(fontFamily = FontFamily.Default)

val EgoSyncTypography = Typography(
    displaySmall = Base.copy(fontSize = 28.sp, lineHeight = 36.sp, fontWeight = FontWeight.SemiBold),
    headlineSmall = Base.copy(fontSize = 22.sp, lineHeight = 28.sp, fontWeight = FontWeight.SemiBold),
    titleLarge = Base.copy(fontSize = 18.sp, lineHeight = 26.sp, fontWeight = FontWeight.SemiBold),
    titleMedium = Base.copy(fontSize = 15.sp, lineHeight = 22.sp, fontWeight = FontWeight.Medium),
    titleSmall = Base.copy(fontSize = 13.sp, lineHeight = 18.sp, fontWeight = FontWeight.Medium),
    bodyLarge = Base.copy(fontSize = 16.sp, lineHeight = 26.sp),
    bodyMedium = Base.copy(fontSize = 14.sp, lineHeight = 22.sp),
    bodySmall = Base.copy(fontSize = 12.sp, lineHeight = 18.sp),
    labelLarge = Base.copy(fontSize = 14.sp, lineHeight = 20.sp, fontWeight = FontWeight.Medium),
    labelMedium = Base.copy(fontSize = 12.sp, lineHeight = 16.sp, fontWeight = FontWeight.Medium),
    labelSmall = Base.copy(fontSize = 10.sp, lineHeight = 14.sp, fontWeight = FontWeight.Medium),
)
