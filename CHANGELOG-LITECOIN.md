# Changelog (LitecoinDevKit bindings)

Litecoin-specific notes for `bdk-ffi` on branch `litecoin-mweb`.  
Upstream-shaped history remains in [`CHANGELOG.md`](CHANGELOG.md); consumers should read **this** file for LDK releases.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [3.1.0-litecoin.1] — 2026-08

### Blessed consumer pin

- Android AAR / xcframework: **`3.1.0-litecoin.1`**
- SwiftPM (`ltc-swift`): **`3.1.0-litecoin.2`** (wraps this binary release)
- Rust `bdk_wallet`: **`3.1.0-litecoin.0`** (rev-pinned)

### Added / exposed (MWEB)

- `MasterKeys`, address helpers (`mwebParseAddress` / `mwebIsMwebAddress`)
- `MwebStore` (memory + SQLite path helpers) and blocking `MwebSyncer`
- `Wallet.balanceCombined`
- Maps-first: `prepareMwebPegin`, `fundMwebSend`, `fundMwebPegout`, `signAndExtractFundedMweb`
- Litecoin `Network` (use `Testnet4`; no Bitcoin-style `Testnet` variant)
- No `bdk_kyoto` (Bitcoin-only)

### Packaging

- GitHub release assets: `bdk-ltc-android.aar`, `bdkFFI.xcframework.zip`
- Primary Android install path: **file AAR** + JNA (Maven Central not required)
- In-tree Gradle `version` may show `*-SNAPSHOT` for local publishes — ignore for consumers
- Release process: [`docs/RELEASE_LADDER.md`](docs/RELEASE_LADDER.md)

### Known limits

- Event-returning wallet APIs from upstream BDK may still be stubbed on the Litecoin wallet fork
- Rev pins must stay coherent across `bdk` ↔ `bdk_wallet` ↔ `bdk-ffi` (`scripts/check-rev-coherence.sh`)
- `litecoin` crates.io `0.32.8-rc.2` may still be git/path-patched upstream of this build

## [3.1.0-litecoin.0] — 2026-07

Initial LitecoinDevKit mobile bindings cut with MWEB façade on UniFFI (Android + Swift).
