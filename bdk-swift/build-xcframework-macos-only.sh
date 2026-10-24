#!/bin/bash
# macOS-only xcframework for local smoke testing when full Xcode iOS SDKs are unavailable.
# For device/simulator slices, use build-xcframework-dev.sh with Xcode.app selected via xcode-select.

set -euo pipefail

HEADER_BASENAME="LitecoinDevKitFFI"
TARGETDIR="../bdk-ffi/target"
OUTDIR="."
NAME="bdkffi"
STATIC_LIB_NAME="libbdkffi.a"
NEW_HEADER_DIR="../bdk-ffi/target/include"
PROFILE_DIR="debug"
SWIFT_OUT_DIR="../bdk-swift/Sources/LitecoinDevKit"
HEADER_OUT_DIR="${NEW_HEADER_DIR}/${HEADER_BASENAME}"

HOST_ARCH=$(uname -m)
if [ "$HOST_ARCH" = "arm64" ]; then
    MAC_TARGET="aarch64-apple-darwin"
else
    MAC_TARGET="x86_64-apple-darwin"
fi

cd ../bdk-ffi/ || exit

rustup target add "$MAC_TARGET"
cargo build --package bdk-ffi --target "$MAC_TARGET"

UNIFFI_LIBRARY_PATH="./target/$MAC_TARGET/$PROFILE_DIR/libbdkffi.dylib"
cargo run --bin uniffi-bindgen generate \
    --library "${UNIFFI_LIBRARY_PATH}" \
    --language swift \
    --out-dir "${SWIFT_OUT_DIR}" \
    --no-format

rm -rf "${NEW_HEADER_DIR:?}"/*
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

# Prefer xcodebuild when available; otherwise assemble a minimal xcframework layout.
if xcodebuild -version >/dev/null 2>&1; then
    xcodebuild -create-xcframework \
        -library "${TARGETDIR}/${MAC_TARGET}/${PROFILE_DIR}/${STATIC_LIB_NAME}" \
        -headers "${NEW_HEADER_DIR}" \
        -output "${OUTDIR}/${NAME}.xcframework"
else
    echo "xcodebuild unavailable; writing manual macOS xcframework layout"
    FRAME="${OUTDIR}/${NAME}.xcframework"
    SLICE="macos-${MAC_TARGET}"
    rm -rf "${FRAME}"
    mkdir -p "${FRAME}/${SLICE}/Headers"
    cp "${TARGETDIR}/${MAC_TARGET}/${PROFILE_DIR}/${STATIC_LIB_NAME}" "${FRAME}/${SLICE}/"
    cp -R "${NEW_HEADER_DIR}/." "${FRAME}/${SLICE}/Headers/"
    cat > "${FRAME}/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>AvailableLibraries</key>
	<array>
		<dict>
			<key>LibraryIdentifier</key>
			<string>${SLICE}</string>
			<key>LibraryPath</key>
			<string>${STATIC_LIB_NAME}</string>
			<key>HeadersPath</key>
			<string>Headers</string>
			<key>SupportedArchitectures</key>
			<array>
				<string>$(echo "$MAC_TARGET" | cut -d- -f1)</string>
			</array>
			<key>SupportedPlatform</key>
			<string>macos</string>
		</dict>
	</array>
	<key>CFBundlePackageType</key>
	<string>XFWK</string>
	<key>XCFrameworkFormatVersion</key>
	<string>1.0</string>
</dict>
</plist>
EOF
fi

echo "Wrote ${OUTDIR}/${NAME}.xcframework"
