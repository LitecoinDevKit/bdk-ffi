#!/usr/bin/env bash
# The bdk.git rev pinned in bdk-ffi/Cargo.toml must be the same rev that the
# pinned bdk_wallet commit pins in its own Cargo.toml. If they diverge, Cargo
# treats them as two different sources and builds two copies of
# bdk_chain/bdk_mweb, and types stop unifying across the API boundary.
set -euo pipefail
cd "$(dirname "$0")/.."

manifest="bdk-ffi/Cargo.toml"

wallet_rev=$(grep -oE 'LitecoinDevKit/bdk_wallet\.git", rev = "[0-9a-f]{40}"' "$manifest" | grep -oE '[0-9a-f]{40}')
if [ -z "$wallet_rev" ]; then
  echo "could not find a rev-pinned bdk_wallet dependency in $manifest" >&2
  exit 1
fi

our_bdk_revs=$(grep -oE 'LitecoinDevKit/bdk\.git", rev = "[0-9a-f]{40}"' "$manifest" | grep -oE '[0-9a-f]{40}' | sort -u)
if [ "$(echo "$our_bdk_revs" | wc -l)" -ne 1 ]; then
  echo "expected exactly one distinct bdk.git rev in $manifest, found:" >&2
  echo "$our_bdk_revs" >&2
  exit 1
fi

wallet_bdk_revs=$(curl -fsSL "https://raw.githubusercontent.com/LitecoinDevKit/bdk_wallet/${wallet_rev}/Cargo.toml" \
  | grep -oE 'LitecoinDevKit/bdk\.git", rev = "[0-9a-f]{40}"' | grep -oE '[0-9a-f]{40}' | sort -u)
if [ "$(echo "$wallet_bdk_revs" | wc -l)" -ne 1 ]; then
  echo "expected exactly one distinct bdk.git rev in bdk_wallet@${wallet_rev}, found:" >&2
  echo "$wallet_bdk_revs" >&2
  exit 1
fi

if [ "$our_bdk_revs" != "$wallet_bdk_revs" ]; then
  echo "bdk.git rev mismatch:" >&2
  echo "  bdk-ffi pins:              $our_bdk_revs" >&2
  echo "  bdk_wallet@${wallet_rev} pins: $wallet_bdk_revs" >&2
  echo "Update the pins so both point at the same bdk commit." >&2
  exit 1
fi

echo "rev coherence OK: bdk.git @ $our_bdk_revs (via bdk_wallet @ $wallet_rev)"
