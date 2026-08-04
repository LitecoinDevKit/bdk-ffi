# Litecoin bdk-ffi

UniFFI language bindings for the [Litecoin BDK fork](https://github.com/IndigoNakamoto/bdk) (with MWEB support), targeting Swift (iOS/macOS) and Kotlin (Android).

Forked from [bitcoindevkit/bdk-ffi](https://github.com/bitcoindevkit/bdk-ffi) and rewired to:

- the Litecoin fork's `bdk_wallet`, `bdk_esplora`, `bdk_electrum`, and `bdk_mweb` crates
- Litecoin `Network` (no `Testnet` variant — use `Testnet4`)
- MWEB façade: combined balance, stealth addresses, peg-in / send / peg-out, `MwebStore`, `MwebSyncer`
- **No** `bdk_kyoto` (Bitcoin-only compact block filters)

## Layout

| Path | Purpose |
|------|---------|
| [`bdk-ffi/`](bdk-ffi/) | Rust UniFFI wrapper crate (`cdylib` / `staticlib`) |
| [`bdk-android/`](bdk-android/) | Kotlin/Android AAR (`org.litecoindevkit:bdk-ltc-android`) |
| [`bdk-swift/`](bdk-swift/) | SwiftPM package `LitecoinDevKit` + xcframework scripts |

## Using the bindings (no Rust required)

Prebuilt artifacts are published on the [GitHub releases page](https://github.com/IndigoNakamoto/bdk-ffi/releases/tag/3.1.0-litecoin.1):

- **Android** — download `bdk-ltc-android.aar`. Drop it into your app's `libs/` and add
  `implementation(files("libs/bdk-ltc-android.aar"))` plus
  `implementation("net.java.dev.jna:jna:5.14.0@aar")`. Kotlin package: `org.litecoindevkit`.
- **Swift** — add the SwiftPM package
  [`IndigoNakamoto/ltc-swift`](https://github.com/IndigoNakamoto/ltc-swift)
  (`.package(url: "https://github.com/IndigoNakamoto/ltc-swift", from: "3.1.0-litecoin.1")`).
  The binary `bdkFFI.xcframework.zip` (iOS device + simulator + macOS) is hosted on the
  bdk-ffi release; `ltc-swift` is the thin Package.swift + generated bindings wrapper.

Releases are produced by the [`Build Release Artifacts`](.github/workflows/release.yaml)
workflow (`workflow_dispatch`), which builds the AAR, the xcframework zip, its SwiftPM
checksum, and a ready-to-commit `Package.swift.release`.

## Building from source

The `bdk-ffi` crate uses path dependencies into the Litecoin BDK monorepo, so it must be
nested inside checkouts of [`IndigoNakamoto/bdk`](https://github.com/IndigoNakamoto/bdk)
and [`IndigoNakamoto/bdk_wallet`](https://github.com/IndigoNakamoto/bdk_wallet):

```shell
git clone --branch litecoin https://github.com/IndigoNakamoto/bdk.git
cd bdk
git clone --branch litecoin https://github.com/IndigoNakamoto/bdk_wallet.git
git clone --branch litecoin-mweb https://github.com/IndigoNakamoto/bdk-ffi.git
```

Resulting layout (CI reconstructs the same layout via
[`.github/actions/setup-workspace`](.github/actions/setup-workspace/action.yml)):

```text
bdk/                 IndigoNakamoto/bdk        (litecoin)
├── crates/          bdk_chain, bdk_esplora, bdk_electrum, bdk_mweb, ...
├── bdk_wallet/      IndigoNakamoto/bdk_wallet (litecoin)
└── bdk-ffi/         this repo                 (litecoin-mweb)
    └── bdk-ffi/     the Rust crate
```

### Rust checks

```shell
cd bdk-ffi/bdk-ffi
cargo build
cargo test
```

### Android

Requires JDK 17, the Android SDK, and the Android NDK.

On macOS: `brew install --cask temurin@17 android-commandlinetools android-ndk`, then
`sdkmanager "platform-tools" "platforms;android-36" "build-tools;36.0.0"` and put
`sdk.dir=<sdk path>` in `bdk-android/local.properties` (or export `ANDROID_HOME`).

```shell
export ANDROID_NDK_ROOT=/opt/homebrew/share/android-ndk   # or ANDROID_NDK_HOME
cd bdk-android
bash ./scripts/release/build-release-macos-aarch64.sh   # arm64 + armv7 + x86_64
./gradlew :lib:assembleRelease        # AAR in lib/build/outputs/aar/
./gradlew :lib:publishToMavenLocal    # or publish locally as org.litecoindevkit:bdk-ltc-android
```

On Linux use `scripts/release/build-release-linux-x86_64.sh` instead.

### Swift / iOS

```shell
cd bdk-swift
# macOS-only smoke build (works with Command Line Tools):
bash ./build-xcframework-macos-only.sh

# Full iOS device + simulator + macOS xcframework requires Xcode.app:
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
bash ./build-xcframework.sh
swift test
```

`Package.swift` in this repo points at the locally built `./bdkFFI.xcframework`, so after
running a build script you can depend on the `bdk-swift` directory as a local SwiftPM package.

```swift
import LitecoinDevKit

let keys = try MasterKeys.fromSeed(seed: seed, network: .bitcoin) // .bitcoin = Litecoin mainnet
let addr = try keys.receiveAddress(index: 0, network: .bitcoin)
```

## Cutting a release

1. Run the `Build Release Artifacts` workflow with the version (e.g. `3.1.0-litecoin.0`).
   It uploads the AAR, `bdkFFI.xcframework.zip`, the generated `LitecoinDevKit.swift`
   bindings, and a `Package.swift.release` pre-filled with the asset URL and checksum,
   and creates a draft GitHub release with the artifacts attached.
2. Publish the draft release so the `bdkFFI.xcframework.zip` asset URL becomes public.
3. For Swift consumers via SwiftPM URL: update
   [`IndigoNakamoto/ltc-swift`](https://github.com/IndigoNakamoto/ltc-swift) with
   `Package.swift.release` (renamed to `Package.swift`) plus
   `Sources/LitecoinDevKit/LitecoinDevKit.swift` from the workflow artifacts, and tag it
   with the release version.

## MWEB API (all languages)

- `MasterKeys` / `mwebParseAddress` / `mwebIsMwebAddress`
- `MwebStore` (in-memory + SQLite path helpers)
- `Wallet.balanceCombined`
- `Wallet.prepareMwebPegin` → sign transparent → broadcast → `insertOutputsIntoStore` after maturity
- `Wallet.fundMwebSend` / `fundMwebPegout` / `signAndExtractFundedMweb`
- `MwebSyncer` — **blocking**; call from a background thread / coroutine dispatcher
