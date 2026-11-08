import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

val rustOut = layout.buildDirectory.dir("generated/rust")

android {
    namespace = "org.openslate.android"
    compileSdk = 37

    defaultConfig {
        applicationId = "org.openslate.android"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            // per installarla sul telefono a mano (al posto della debug): stessa chiave
            signingConfig = signingConfigs.getByName("debug")
            // solo i telefoni: la versione x86_64 serve all'emulatore
            ndk { abiFilters += "arm64-v8a" }
        }
    }
    packaging {
        // la libreria Rust compressa nell'APK: da scaricare pesa molto meno
        jniLibs.useLegacyPackaging = true
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures {
        compose = true
    }
    // Generati da build-rust.sh (task buildRust, prima di preBuild).
    sourceSets["main"].kotlin.directories.add("build/generated/rust/kotlin")
    sourceSets["main"].jniLibs.directories.add("build/generated/rust/jniLibs")
}

// Il cuore Rust: mobile/core (condiviso con iOS) più i file condivisi con la desktop (src/).
val buildRust = tasks.register<Exec>("buildRust") {
    val sdk = Properties().apply {
        rootProject.file("local.properties").takeIf { it.exists() }?.inputStream()?.use { load(it) }
    }.getProperty("sdk.dir")
    if (sdk != null) environment("ANDROID_HOME", sdk)
    workingDir = rootDir
    commandLine("sh", "build-rust.sh")
    inputs.dir(rootProject.file("../mobile/core/src"))
    inputs.file(rootProject.file("../mobile/core/Cargo.toml"))
    inputs.files(
        rootProject.file("../src/protocol.rs"),
        rootProject.file("../src/live.rs"),
        rootProject.file("../src/stroke.rs"),
    )
    outputs.dir(rustOut)
}
tasks.named("preBuild") { dependsOn(buildRust) }

dependencies {
    implementation(platform("androidx.compose:compose-bom:2026.09.00"))
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    // le icone (nella versione per il telefono R8 tiene solo quelle usate)
    implementation("androidx.compose.material:material-icons-extended:1.7.8")
    // Il QR code delle impostazioni AI della desktop (ZXing, Apache-2.0: senza i servizi Google)
    implementation("com.journeyapps:zxing-android-embedded:4.3.0")
    // I binding UniFFI caricano la libreria Rust con JNA.
    implementation("net.java.dev.jna:jna:5.17.0@aar")
}
