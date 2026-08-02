# bdk-ltc-android

Kotlin / Android bindings for the Litecoin BDK fork (with MWEB).

```kotlin
repositories { mavenLocal() /* or your publish target */ }

dependencies {
    implementation("org.litecoindevkit:bdk-ltc-android:<version>")
}
```

## Build

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
