plugins {
    alias(libs.plugins.android.application)
    // AGP 9 内置 Kotlin 编译（org.jetbrains.kotlin.android 已不再需要）
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.egosync.companion"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.egosync.companion"
        minSdk = 26
        targetSdk = 37
        versionCode = 2
        versionName = "0.1.6-alpha.1"
        // Story 12.4 Task 8：androidTest 冒烟需 androidx instrumented runner
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
    }

    testOptions {
        unitTests {
            // P7 编排测试在 JVM 跑真实连接客户端：成功/失败路径都会打
            // CompanionLog（android.util.Log），stub 默认抛 "not mocked" 会让
            // 日志调用伪装成连接失败——此处降级为 no-op 返回默认值
            isReturnDefaultValues = true
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

// 单一事实源：12.1 黄金向量 fixture 由 companion-proto crate 复制而来，禁止手工拷贝副本。
val copyNoiseVectors = tasks.register<Copy>("copyNoiseVectors") {
    from(rootDir.resolve("../crates/companion-proto/tests/fixtures/noise_java_vectors.json"))
    into(layout.projectDirectory.dir("src/test/resources"))
}

dependencies {
    implementation(platform(libs.compose.bom))
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.lifecycle.viewmodel.ktx)
    implementation(libs.androidx.navigation.compose)

    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.material.icons.core)

    implementation(libs.kotlinx.coroutines.android)

    implementation(libs.noise.java)
    implementation(libs.mlkit.barcode.scanning)
    implementation(libs.okhttp)

    debugImplementation(libs.androidx.compose.ui.tooling)

    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.org.json)

    // Story 12.4 Task 8：androidTest 冒烟（真 Keystore 路径 + on-device noise-java 互通）
    androidTestImplementation(libs.junit)
    androidTestImplementation(libs.kotlinx.coroutines.test)
    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.androidx.test.runner)
}

// 复制任务写入 src/test/resources，所有消费该目录的任务（JavaRes 处理 + 测试）须显式依赖。
tasks.matching {
    it.name in setOf("processDebugUnitTestJavaRes", "processReleaseUnitTestJavaRes")
}.configureEach {
    dependsOn(copyNoiseVectors)
}

tasks.withType<Test>().configureEach {
    dependsOn(copyNoiseVectors)
}
