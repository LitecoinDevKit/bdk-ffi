# Litecoin bdk-ffi

UniFFI language bindings for the [Litecoin BDK fork](https://github.com/LitecoinDevKit/bdk) (with MWEB support), targeting Swift (iOS/macOS) and Kotlin (Android).

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

## Blessed version matrix

**Canonical pin table (source of truth):**  
[`LitecoinDevKit/bdk` docs/ADOPTION.md § Blessed consumer pin](https://github.com/LitecoinDevKit/bdk/blob/litecoin/docs/ADOPTION.md#blessed-consumer-pin-this-phase--source-of-truth).  
Update pins there first; this summary must not drift.

| Component | Version | Role |
| --- | --- | --- |
| **This repo / UniFFI crate** | `3.2.0-litecoin.0` (source, `litecoin-mweb`) | Source of the next AAR + xcframework; tracks upstream `bdk-ffi` master `ec61c0a` |
| **Android AAR** (`bdk-ltc-android.aar`) | `3.1.0-litecoin.3` | [GitHub release asset](https://github.com/LitecoinDevKit/bdk-ffi/releases/tag/3.1.0-litecoin.3) |
| **xcframework zip** | `3.1.0-litecoin.3` | Same release |
| **`ltc-swift` SwiftPM tag** | `3.1.0-litecoin.3` | Thin wrapper over the `3.1.0-litecoin.3` xcframework (tags re-synced at `.3`) |
| **`bdk_wallet` (Rust)** | `3.2.0-litecoin.0` | Rev-pinned by this crate on `litecoin-mweb`; the `3.1.0-litecoin.3` binaries above were built against `3.1.0-litecoin.0` |
| **Gradle `version` in-tree** | `3.2.0-litecoin.0-SNAPSHOT` | Local/dev publish only — **not** the consumer pin |

**Blessed consumer pin for this phase:** Android apps use AAR **`3.1.0-litecoin.3`**; Swift apps use `ltc-swift` **exact `3.1.0-litecoin.3`**. Both resolve to the same `3.1.0-litecoin.3` binaries.

Litecoin release notes: [`CHANGELOG-LITECOIN.md`](CHANGELOG-LITECOIN.md).  
Cutting a release: [`docs/RELEASE_LADDER.md`](docs/RELEASE_LADDER.md).  
Integrator overview + dogfood: [ADOPTION.md](https://github.com/LitecoinDevKit/bdk/blob/litecoin/docs/ADOPTION.md) · [DOGFOOD_CHECKLIST.md](https://github.com/LitecoinDevKit/bdk/blob/litecoin/docs/DOGFOOD_CHECKLIST.md).

## Using the bindings (no Rust required)

Prebuilt artifacts are published on the [GitHub releases page](https://github.com/LitecoinDevKit/bdk-ffi/releases/tag/3.1.0-litecoin.3):

- **Android (primary path)** — download `bdk-ltc-android.aar`. Drop it into your app's `libs/` and add
  `implementation(files("libs/bdk-ltc-android.aar"))` plus
  `implementation("net.java.dev.jna:jna:5.14.0@aar")`. Kotlin package: `org.litecoindevkit`.
  Do **not** require Maven Central for Litecoin builds; the in-repo Gradle coordinate
  `3.1.0-litecoin.3-SNAPSHOT` is for local `publishToMavenLocal` only.
- **Swift** — add the SwiftPM package
  [`LitecoinDevKit/ltc-swift`](https://github.com/LitecoinDevKit/ltc-swift)
  (`.package(url: "https://github.com/LitecoinDevKit/ltc-swift", exact: "3.1.0-litecoin.3")`).
  The binary `bdkFFI.xcframework.zip` (iOS device + simulator + macOS) is hosted on the
  bdk-ffi release; `ltc-swift` is the thin Package.swift + generated bindings wrapper.

Releases are produced by the [`Build Release Artifacts`](.github/workflows/release.yaml)
workflow (`workflow_dispatch`), which builds the AAR, the xcframework zip, its SwiftPM
checksum, and a ready-to-commit `Package.swift.release`. Full checklist:
[`docs/RELEASE_LADDER.md`](docs/RELEASE_LADDER.md).

## Building from source

The fork dependencies ([`LitecoinDevKit/bdk`](https://github.com/LitecoinDevKit/bdk),
[`bdk_wallet`](https://github.com/LitecoinDevKit/bdk_wallet),
[`rust-litecoin`](https://github.com/LitecoinDevKit/rust-litecoin)) are rev-pinned git
dependencies in [`bdk-ffi/Cargo.toml`](bdk-ffi/Cargo.toml) with a committed `Cargo.lock`,
so a plain clone builds standalone:

```shell
git clone --branch litecoin-mweb https://github.com/LitecoinDevKit/bdk-ffi.git
```

The `bdk.git` rev pinned here must match the one the pinned `bdk_wallet` commit pins in
its own manifest — otherwise Cargo builds two copies of `bdk_chain`/`bdk_mweb` and types
stop unifying. [`scripts/check-rev-coherence.sh`](scripts/check-rev-coherence.sh) verifies
this (CI runs it on every build and release).

### Rust checks

```shell
cd bdk-ffi
cargo build
cargo test
```

### Local development against sibling checkouts

To hack on `bdk`/`bdk_wallet` and this crate together without pushing pins, clone them as
siblings of this repo and add a gitignored `.cargo/config.toml` at the repo root:

```toml
[patch."https://github.com/LitecoinDevKit/bdk.git"]
bdk_electrum = { path = "../bdk/crates/electrum" }
bdk_esplora = { path = "../bdk/crates/esplora" }
bdk_mweb = { path = "../bdk/crates/mweb" }
bdk_chain = { path = "../bdk/crates/chain" }

[patch."https://github.com/LitecoinDevKit/bdk_wallet.git"]
bdk_wallet = { path = "../bdk_wallet" }
```

Remove it (or `rm .cargo/config.toml`) to build against the pinned revs again. The
committed manifests always keep the pins.

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

`Package.swift` in this repo points at the locally built `./bdkffi.xcframework`, so after
running a build script you can depend on the `bdk-swift` directory as a local SwiftPM package.

```swift
import LitecoinDevKit

let keys = try MasterKeys.fromSeed(seed: seed, network: .bitcoin) // .bitcoin = Litecoin mainnet
let addr = try keys.receiveAddress(index: 0, network: .bitcoin)
```

## Security Policy

To report a security issue, please refer to the [security policy](SECURITY.md). Issues
specific to the Litecoin/MWEB surface can also be filed as a private advisory on this
repository.

## Cutting a release

See the full ladder in [`docs/RELEASE_LADDER.md`](docs/RELEASE_LADDER.md). Short form:

1. Bump rev pins (`bdk` / `bdk_wallet`) in lockstep; run `scripts/check-rev-coherence.sh`.
2. Run `Build Release Artifacts` (`workflow_dispatch`) with the version (e.g. `3.1.0-litecoin.3`).
3. Publish the draft GitHub release.
4. Update [`ltc-swift`](https://github.com/LitecoinDevKit/ltc-swift) from workflow artifacts and tag.
5. Bump smoke apps (`ltc-wallet-android` / `ltc-wallet-ios`) to the blessed pin.
6. Append notes in [`CHANGELOG-LITECOIN.md`](CHANGELOG-LITECOIN.md).

## MWEB API (all languages)

- `MasterKeys` / `mwebParseAddress` / `mwebIsMwebAddress`
- `MwebStore` (in-memory + SQLite path helpers)
- `Wallet.balanceCombined`
- `Wallet.prepareMwebPegin` → sign transparent → broadcast → `insertOutputsIntoStore` after maturity
- `Wallet.fundMwebSend` / `fundMwebPegout` / `signAndExtractFundedMweb`
- `MwebSyncer` — **blocking**; call from a background thread / coroutine dispatcher
