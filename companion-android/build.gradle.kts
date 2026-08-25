// 根构建脚本：单模块 :app，插件统一在 app 模块声明
// 注：AGP 9 内置 Kotlin 编译，无需 org.jetbrains.kotlin.android 插件
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.compose) apply false
}
