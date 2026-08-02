import Foundation
import LitecoinDevKit

/// Local smoke example: derive an MWEB receive address and print combined balance of an empty wallet.
///
/// Build the macOS xcframework first:
///   cd bdk-swift && bash ./build-xcframework-macos-only.sh
/// Then from an Xcode/SwiftPM consumer that depends on this package.

let seed = Data(repeating: 0x11, count: 32)
let keys = try MasterKeys.fromSeed(seed: seed, network: .bitcoin)
let addr = try keys.receiveAddress(index: 0, network: .bitcoin)
print("MWEB receive address[0]: \(addr)")
print("is_mweb: \(mwebIsMwebAddress(address: addr))")

let store = MwebStore()
print("MWEB unspent count: \(store.unspentCount())")
