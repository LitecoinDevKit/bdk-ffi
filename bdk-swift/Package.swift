// swift-tools-version:5.5
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "LitecoinDevKit",
    platforms: [
        .macOS(.v12),
        .iOS(.v15)
    ],
    products: [
        .library(
            name: "LitecoinDevKit",
            targets: ["bdkFFI", "LitecoinDevKit"]),
    ],
    dependencies: [],
    targets: [
        // Built locally via build-xcframework-dev.sh / build-xcframework.sh
        .binaryTarget(name: "bdkFFI", path: "./bdkFFI.xcframework"),
        .target(
            name: "LitecoinDevKit",
            dependencies: ["bdkFFI"]
        ),
        .testTarget(
            name: "LitecoinDevKitTests",
            dependencies: ["LitecoinDevKit"]
        ),
    ]
)
