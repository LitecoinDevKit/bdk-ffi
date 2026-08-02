# LitecoinDevKit (bdk-swift)

Swift language bindings for the Litecoin BDK fork (with MWEB), generated via [UniFFI](https://mozilla.github.io/uniffi-rs/) from [`bdk-ffi`](../bdk-ffi).

Supported targets (when built with full Xcode):

- macOS (Apple Silicon / Intel)
- iOS device (`aarch64-apple-ios`)
- iOS simulator (`aarch64-apple-ios-sim`)

## Package name

```swift
import LitecoinDevKit
```

Maven-style coordinates for the sister Android package: `org.litecoindevkit:bdk-ltc-android`.

## Build locally

### Prerequisites

- Rust toolchain (see `bdk-ffi/rust-toolchain.toml`)
- For **iOS device/simulator slices**: full **Xcode.app** (not only Command Line Tools), with iPhoneOS / iPhoneSimulator SDKs:
  ```shell
  sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
  ```
- For a **macOS-only** smoke framework (no iOS SDK):
  ```shell
  cd bdk-swift
  bash ./build-xcframework-macos-only.sh
  ```

### Full xcframework (macOS + iOS + sim)

```shell
cd bdk-swift
bash ./build-xcframework-dev.sh
swift test
```

## MWEB smoke

See [`Examples/MwebSmoke/main.swift`](Examples/MwebSmoke/main.swift) and
[`Tests/LitecoinDevKitTests/MwebSmokeTests.swift`](Tests/LitecoinDevKitTests/MwebSmokeTests.swift).

Core APIs:

- `MasterKeys.fromSeed` / `receiveAddress`
- `MwebStore` / `Wallet.balanceCombined`
- `Wallet.prepareMwebPegin` / `fundMwebSend` / `fundMwebPegout` / `signAndExtractFundedMweb`
- `MwebSyncer` (blocking; call off the main thread)

## Note on this machine

If `xcodebuild` reports that only Command Line Tools are active, iOS SDK builds will fail until Xcode is installed and selected. Android bindings do not require Xcode.
