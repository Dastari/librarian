import java.util.Properties

plugins {
    id("com.android.application")
}

// One source for the version: ../version.properties. versionCode = major * 10000 + minor * 100 + patch.
val versionProps = Properties().apply { rootProject.file("version.properties").inputStream().use { load(it) } }
val appVersionName: String = versionProps.getProperty("versionName").trim()
val appVersionCode: Int = appVersionName.split(".").map { it.toInt() }.let { (major, minor, patch) ->
    require(minor < 100 && patch < 100) { "versionName parts above 99 do not fit the versionCode scheme" }
    major * 10000 + minor * 100 + patch
}

// The release key lives outside the repository. Its properties file (storeFile, storePassword,
// keyAlias, keyPassword) is read from LIBRARIAN_TV_KEYSTORE_PROPERTIES, else
// /root/.config/librarian/android-tv-keystore.properties. Without it, release builds are signed
// with the debug key so they still install for testing (but cannot update a properly signed one).
val keystorePropsFile = file(
    System.getenv("LIBRARIAN_TV_KEYSTORE_PROPERTIES") ?: "/root/.config/librarian/android-tv-keystore.properties",
)
val keystoreProps: Properties? = keystorePropsFile.takeIf { it.canRead() }?.let { f ->
    Properties().apply { f.inputStream().use { load(it) } }
}

android {
    namespace = "net.dastari.librarian.tv"
    compileSdk = 37

    defaultConfig {
        applicationId = "net.dastari.librarian.tv"
        minSdk = 24
        targetSdk = 37
        versionCode = appVersionCode
        versionName = appVersionName
    }

    signingConfigs {
        if (keystoreProps != null) {
            create("release") {
                storeFile = file(keystoreProps.getProperty("storeFile"))
                storePassword = keystoreProps.getProperty("storePassword")
                keyAlias = keystoreProps.getProperty("keyAlias")
                keyPassword = keystoreProps.getProperty("keyPassword")
                enableV1Signing = true
                enableV2Signing = true
                enableV3Signing = true
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
        }
        debug {
            applicationIdSuffix = ".debug"
            versionNameSuffix = "-debug"
        }
    }

    buildFeatures {
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    lint {
        // Release builds skip the lint pass to keep builds light; run `lintDebug` on purpose.
        checkReleaseBuilds = false
        abortOnError = true
    }

    testOptions {
        unitTests.isReturnDefaultValues = true
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}

// `./gradlew distApks` copies the APKs under their release names to build/dist.
val distApks by tasks.registering(Copy::class) {
    dependsOn("assembleRelease", "assembleDebug")
    from(layout.buildDirectory.file("outputs/apk/release/app-release.apk")) {
        rename { "librarian-tv-$appVersionName.apk" }
    }
    from(layout.buildDirectory.file("outputs/apk/debug/app-debug.apk")) {
        rename { "librarian-tv-$appVersionName-debug.apk" }
    }
    into(layout.buildDirectory.dir("dist"))
}
