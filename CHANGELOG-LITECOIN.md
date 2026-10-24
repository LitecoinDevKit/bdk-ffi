# Changelog (LitecoinDevKit bindings)

Litecoin-specific notes for `bdk-ffi` on branch `litecoin-mweb`.  
Upstream-shaped history remains in [`CHANGELOG.md`](CHANGELOG.md); consumers should read **this** file for LDK releases.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [3.1.0-litecoin.3] — 2026-10

`3.1.0-litecoin.2` was never cut as a binary: `ltc-swift` used that number for a
URL-only wrapper bump over the `.1` xcframework. Binary and SwiftPM tags are
re-synced at `.3` and move together from here on.

### Blessed consumer pin

- Android AAR / xcframework: **`3.1.0-litecoin.3`**
- SwiftPM (`ltc-swift`): **`3.1.0-litecoin.3`**
- Rust `bdk_wallet`: **`3.1.0-litecoin.0`** @ `f1307827` (pins `bdk` `2deac37e`)

### Security

- rustls 0.21.12, rustls-webpki 0.101.7 and webpki-roots 0.25.4 are gone from the
  shipped binaries: RUSTSEC-2026-0098 / 0099 / 0104 / 0346 / 0349 / 0381 / 0382.
  The esplora client fork moved to `minreq` 3 and the electrum client fork to
  `webpki-roots` 1.x; the runtime TLS stack is a single rustls 0.23.46.
- rustls' `aws-lc-rs` provider is now linked alongside `ring`. The esplora client
  installs a process-level `CryptoProvider` on construction when none is set, so
  opening an Esplora client before an Electrum one no longer panics.

### Changed

- Rebased on the `bitcoindevkit/bdk` `7b68d280` merge (`bdk` `2deac37e`,
  `bdk_wallet` `f1307827`): `Wallet.balance` is taint-aware
  (`CanonicalView::balance`), `bdk_chain` 0.23.2 / `bdk_core` 0.6.2.
- Release workflow: `android-actions/setup-android@v4` (Google dropped the
  `tools` package that `v3` required).

### Known limits

- Unchanged from `3.1.0-litecoin.1` below.

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
