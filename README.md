# Litecoin bdk-ffi

UniFFI language bindings for the [IndigoNakamoto Litecoin BDK fork](https://github.com/IndigoNakamoto/bdk) (with MWEB).

Forked from [bitcoindevkit/bdk-ffi](https://github.com/bitcoindevkit/bdk-ffi) and rewired to:

- path deps on this monorepo’s `bdk_wallet`, `bdk_esplora`, `bdk_electrum`, `bdk_mweb`
- Litecoin `Network` (no `Testnet` variant — use `Testnet4`)
- MWEB façade: combined balance, stealth addresses, peg-in / send / peg-out, `MwebStore`, `MwebSyncer`
- **No** `bdk_kyoto` (Bitcoin-only CBF)

## Layout

| Path | Purpose |
|------|---------|
| [`bdk-ffi/`](bdk-ffi/) | Rust UniFFI wrapper crate (`cdylib` / `staticlib`) |
| [`bdk-android/`](bdk-android/) | Kotlin/Android AAR (`org.litecoindevkit:bdk-ltc-android`) |
| [`bdk-swift/`](bdk-swift/) | SwiftPM package `LitecoinDevKit` + xcframework scripts |

This directory lives nested under the Litecoin BDK repo (gitignored there) with remotes:

- `upstream` → `https://github.com/bitcoindevkit/bdk-ffi.git`
- `origin` → `https://github.com/IndigoNakamoto/bdk-ffi.git`

## Host (macOS) Rust checks

```shell
cd bdk-ffi
cargo build
cargo test
```

## Android

Requires Android NDK (Homebrew: `brew install --cask android-ndk`).

```shell
export ANDROID_NDK_ROOT=/opt/homebrew/share/android-ndk   # or ANDROID_NDK_HOME
cd bdk-android
bash ./scripts/release/build-release-macos-aarch64.sh   # arm64 + armv7 + x86_64
./gradlew :lib:assembleRelease
```

Kotlin package: `org.litecoindevkit`.

## Swift / iOS

```shell
cd bdk-swift
# macOS-only smoke (works with Command Line Tools):
bash ./build-xcframework-macos-only.sh

# Full iOS device + simulator slices require Xcode.app:
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
bash ./build-xcframework-dev.sh
swift test
```

```swift
import LitecoinDevKit

let keys = try MasterKeys.fromSeed(seed: seed, network: .bitcoin)
let addr = try keys.receiveAddress(index: 0, network: .bitcoin)
```

## MWEB API (all languages)

- `MasterKeys` / `mwebParseAddress` / `mwebIsMwebAddress`
- `MwebStore` (in-memory + SQLite path helpers)
- `Wallet.balanceCombined`
- `Wallet.prepareMwebPegin` → sign transparent → broadcast → `insertOutputsIntoStore` after maturity
- `Wallet.fundMwebSend` / `fundMwebPegout` / `signAndExtractFundedMweb`
- `MwebSyncer` — **blocking**; call from a background thread / coroutine dispatcher
