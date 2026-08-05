# bdk-ltc-android

Kotlin / Android bindings for the Litecoin BDK fork (with MWEB).

## Consuming a release (primary path)

Download the AAR from the [bdk-ffi GitHub release](https://github.com/LitecoinDevKit/bdk-ffi/releases/tag/3.1.0-litecoin.1)
(**blessed pin:** `3.1.0-litecoin.1`):

```kotlin
// app/build.gradle.kts
dependencies {
    implementation(files("libs/bdk-ltc-android.aar"))
    implementation("net.java.dev.jna:jna:5.14.0@aar")
}
```

Kotlin package: `org.litecoindevkit`.

You do **not** need Maven Central for Litecoin builds. The in-tree Gradle
`version = "3.1.0-litecoin.1-SNAPSHOT"` is only for local `publishToMavenLocal` /
dev iteration. Prefer the GitHub release AAR for apps and smoke tests.

See also the root [README version matrix](../README.md#blessed-version-matrix) and
[LitecoinDevKit adoption guide](https://github.com/LitecoinDevKit/bdk/blob/litecoin/docs/ADOPTION.md).

### Optional: mavenLocal

```kotlin
repositories { mavenLocal() }
dependencies {
    implementation("org.litecoindevkit:bdk-ltc-android:3.1.0-litecoin.1-SNAPSHOT")
    implementation("net.java.dev.jna:jna:5.14.0@aar")
}
```

## Build from source

1. Install Android SDK + NDK (API 24+). Homebrew: `brew install --cask android-ndk`.
2. Export NDK path:
   ```shell
   export ANDROID_NDK_ROOT=/opt/homebrew/share/android-ndk
   ```
3. Build native libs + Kotlin bindings:
   ```shell
   cd bdk-android
   bash ./scripts/release/build-release-macos-aarch64.sh
   ./gradlew :lib:assembleRelease
   ```

Dev (arm64 only):

```shell
just build-dev
```

## MWEB

See `lib/src/androidTest/kotlin/org/litecoindevkit/MwebSmokeTest.kt`. Long-running
`MwebSyncer` calls block the thread — use a background dispatcher.

Maps-first surface: `prepareMwebPegin`, `fundMwebSend` / `fundMwebPegout`,
`signAndExtractFundedMweb`, `balanceCombined`.
