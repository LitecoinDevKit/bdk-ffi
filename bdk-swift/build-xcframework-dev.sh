#!/bin/bash
# Dev build of LitecoinDevKit xcframework (macOS + iOS device + iOS sim).
# Requires full Xcode with iPhoneOS / iPhoneSimulator SDKs installed.

set -euo pipefail

HEADER_BASENAME="LitecoinDevKitFFI"
TARGETDIR="../bdk-ffi/target"
OUTDIR="."
NAME="bdkFFI"
STATIC_LIB_NAME="libbdkffi.a"
NEW_HEADER_DIR="../bdk-ffi/target/include"
PROFILE_DIR="debug"
SWIFT_OUT_DIR="../bdk-swift/Sources/LitecoinDevKit"
HEADER_OUT_DIR="${NEW_HEADER_DIR}/${HEADER_BASENAME}"
MIN_IOS_VERSION="15.0"

HOST_ARCH=$(uname -m)
if [ "$HOST_ARCH" = "arm64" ]; then
    MAC_TARGET="aarch64-apple-darwin"
    IOS_SIM_TARGET="aarch64-apple-ios-sim"
else
    MAC_TARGET="x86_64-apple-darwin"
    IOS_SIM_TARGET="x86_64-apple-ios"
fi
IOS_DEVICE_TARGET="aarch64-apple-ios"

export IPHONEOS_DEPLOYMENT_TARGET="${MIN_IOS_VERSION}"

cd ../bdk-ffi/ || exit

rustup component add rust-src
rustup target add "$MAC_TARGET" "$IOS_SIM_TARGET" "$IOS_DEVICE_TARGET"

cargo build --package bdk-ffi --target "$MAC_TARGET"
cargo build --package bdk-ffi --target "$IOS_SIM_TARGET"
cargo build --package bdk-ffi --target "$IOS_DEVICE_TARGET"

UNIFFI_LIBRARY_PATH="./target/$MAC_TARGET/$PROFILE_DIR/libbdkffi.dylib"
cargo run --bin uniffi-bindgen generate \
    --library "${UNIFFI_LIBRARY_PATH}" \
    --language swift \
    --out-dir "${SWIFT_OUT_DIR}" \
    --no-format

rm -rf "${NEW_HEADER_DIR:?}"/*
rm -rf "${HEADER_OUT_DIR:?}"
mkdir -p "${HEADER_OUT_DIR}"
cargo run --bin uniffi-bindgen generate \
    --library "${UNIFFI_LIBRARY_PATH}" \
    --language swift \
    --out-dir "${HEADER_OUT_DIR}" \
    --no-format

find "${HEADER_OUT_DIR}" -maxdepth 1 -name '*.swift' -delete

if [ -f "${HEADER_OUT_DIR}/${HEADER_BASENAME}.modulemap" ]; then
    mv "${HEADER_OUT_DIR}/${HEADER_BASENAME}.modulemap" "${HEADER_OUT_DIR}/module.modulemap"
fi

echo -e "\n" >> "${HEADER_OUT_DIR}/module.modulemap"

rm -f "${SWIFT_OUT_DIR}/${HEADER_BASENAME}.h"
rm -f "${SWIFT_OUT_DIR}/${HEADER_BASENAME}.modulemap"

cd ../bdk-swift/ || exit

rm -rf "${OUTDIR}/${NAME}.xcframework"

xcodebuild -create-xcframework \
    -library "${TARGETDIR}/${MAC_TARGET}/${PROFILE_DIR}/${STATIC_LIB_NAME}" \
    -headers "${NEW_HEADER_DIR}" \
    -library "${TARGETDIR}/${IOS_DEVICE_TARGET}/${PROFILE_DIR}/${STATIC_LIB_NAME}" \
    -headers "${NEW_HEADER_DIR}" \
    -library "${TARGETDIR}/${IOS_SIM_TARGET}/${PROFILE_DIR}/${STATIC_LIB_NAME}" \
    -headers "${NEW_HEADER_DIR}" \
    -output "${OUTDIR}/${NAME}.xcframework"
