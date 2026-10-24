# LitecoinDevKit FFI release ladder

Checklist for shipping `3.1.0-litecoin.N` Android AAR + Swift xcframework.

## Preconditions

- [ ] `bdk` (`litecoin`) and `bdk_wallet` (`litecoin`) commits you intend to ship are pushed.
- [ ] Local siblings (if any) match the revs you will pin — or you pin published git SHAs only.
- [ ] No intentional dual `bdk_chain` / `bdk_mweb` graphs (coherence must pass).

## 1. Bump pins in `bdk-ffi`

Edit [`bdk-ffi/Cargo.toml`](../bdk-ffi/Cargo.toml):

- `bdk_wallet` → new rev + version if wallet tag moved
- `bdk_esplora` / `bdk_electrum` / `bdk_mweb` → same `bdk` rev as wallet’s pin

Bump crate / package versions if cutting a new consumer tag:

- [`bdk-ffi/Cargo.toml`](../bdk-ffi/Cargo.toml) `version`
- [`bdk-android/lib/build.gradle.kts`](../bdk-android/lib/build.gradle.kts) `version` (keep `-SNAPSHOT` suffix for in-tree Gradle; release asset name uses the workflow `version` input)

```bash
cd bdk-ffi
cargo update -p bdk_wallet -p bdk_mweb -p bdk_electrum -p bdk_esplora
bash ./scripts/check-rev-coherence.sh
cargo test
```

## 2. CI green

- [ ] `rust-ci.yaml` (includes rev-coherence)
- [ ] `test-android.yaml` (AAR assemble)
- [ ] `test-swift.yaml` (xcframework + `swift test`)

## 3. Build release artifacts

GitHub → Actions → **Build Release Artifacts** → `workflow_dispatch`

- Input `version`: e.g. `3.1.0-litecoin.1` (no `v` prefix; must match intended tag)
- Wait for draft release with:
  - `bdk-ltc-android.aar`
  - `bdkFFI.xcframework.zip`
  - `LitecoinDevKit.swift`
  - `Package.swift.release`

## 4. Publish draft

- [ ] Review assets / checksum in the draft
- [ ] Publish the GitHub release (makes xcframework URL public for SwiftPM)

## 5. Update `ltc-swift`

In [`LitecoinDevKit/ltc-swift`](https://github.com/LitecoinDevKit/ltc-swift):

1. Replace `Package.swift` with workflow `Package.swift.release`
2. Replace `Sources/LitecoinDevKit/LitecoinDevKit.swift`
3. Tag — the same number as the binary. (A wrapper-only bump happened once: `ltc-swift` `.2`
   pointed at the `.1` zip. The binary skipped `.2` so both tags re-synced at `.3`; keep them
   in lockstep from here on.)
4. Push tag

## 6. Bump smoke / reference consumers

| Repo | What to bump |
| --- | --- |
| `ltc-wallet-android` | `scripts/fetch-aar.sh` default `VERSION` + README |
| `ltc-wallet-ios` | `Package.swift` `exact:` `ltc-swift` tag + README |
| Docs | Update pin table in [`ADOPTION.md`](https://github.com/LitecoinDevKit/bdk/blob/litecoin/docs/ADOPTION.md) **first** (source of truth); then mirror summary in this repo README |

## 7. Changelog

Append a section to [`CHANGELOG-LITECOIN.md`](../CHANGELOG-LITECOIN.md) (do not rely on upstream-shaped `CHANGELOG.md` alone).

## Dry-run notes (2026-08-04)

Local verification without cutting a new GitHub release:

```bash
# from bdk-ffi repo root
bash ./scripts/check-rev-coherence.sh
```

Observed:

```text
rev coherence OK: bdk.git @ 85eb2a996fbc2569746497b2ad87aff3704f06ca (via bdk_wallet @ c0c26dafeea11160f42fce2faf164101191c3912)
```

Full `workflow_dispatch` creates a **draft** release; only run when intentionally cutting
`3.1.0-litecoin.N`. Process rehearsal = steps 1–2 + coherence; skip publishing a draft when no
new artifacts are needed.

**Blessed pin at dry-run time:** AAR/xcframework `3.1.0-litecoin.1`, `ltc-swift` `3.1.0-litecoin.2`,
`bdk_wallet` `3.1.0-litecoin.0`.

## Release notes (2026-10-24, `3.1.0-litecoin.3`)

Full ladder run from `bdk-ffi` `ddf70619` (`bdk_wallet` `f1307827` / `bdk` `2deac37e`).
`release.yaml` needed `android-actions/setup-android@v4` (same breakage `test-android` hit).
Current blessed pin: AAR/xcframework `3.1.0-litecoin.3`, `ltc-swift` `3.1.0-litecoin.3`,
`bdk_wallet` `3.1.0-litecoin.0`.
