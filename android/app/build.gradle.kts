import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

// CI passes the release version; local builds get a dev version.
val scrapVersion = System.getenv("SCRAP_VERSION") ?: "0.0.0-dev"
val scrapVersionCode = System.getenv("SCRAP_VERSION_CODE")?.toIntOrNull() ?: 1
val keystore = System.getenv("SCRAP_KEYSTORE")

android {
    namespace = "dev.plaintextlab.scrap"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.plaintextlab.scrap"
        minSdk = 26
        targetSdk = 35
        versionCode = scrapVersionCode
        versionName = scrapVersion
        ndk { abiFilters += "arm64-v8a" }
    }

    signingConfigs {
        if (keystore != null) {
            create("release") {
                storeFile = file(keystore)
                storePassword = System.getenv("SCRAP_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("SCRAP_KEY_ALIAS") ?: "scrap"
                keyPassword = System.getenv("SCRAP_KEYSTORE_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures { compose = true }
}

kotlin {
    compilerOptions { jvmTarget.set(JvmTarget.JVM_17) }
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2024.12.01"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.work:work-runtime-ktx:2.10.0")
    implementation("com.mikepenz:multiplatform-markdown-renderer-m3:0.30.0")
    // Images in notes: Coil loads them for the renderer (local files, plus web images).
    implementation("com.mikepenz:multiplatform-markdown-renderer-coil3:0.30.0")
    implementation("io.coil-kt.coil3:coil-network-okhttp:3.0.4")
}
