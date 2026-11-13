plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.xekchansky.strangeattractor"
    compileSdk = 35
    // AGP 8.7 defaults to build-tools 34; the CI image installs this one.
    buildToolsVersion = "35.0.0"

    defaultConfig {
        applicationId = "com.xekchansky.strangeattractor"
        minSdk = 26
        targetSdk = 35
        versionCode = 2
        versionName = "2.0.0"
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

    kotlinOptions {
        jvmTarget = "17"
    }

    // The Kotlin sources live in `src/main/kotlin`, the layout the rest of the
    // repository uses.
    sourceSets.getByName("main") {
        java.srcDirs("src/main/kotlin")
    }

    lint {
        abortOnError = true
    }
}
