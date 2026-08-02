import XCTest
@testable import LitecoinDevKit

/// Minimal smoke coverage for the Litecoin/MWEB UniFFI surface.
/// Full wallet sync tests require network + Xcode-linked xcframework.
final class MwebSmokeTests: XCTestCase {
    func testMasterKeysDeriveMwebAddress() throws {
        // 32 bytes of deterministic seed material
        let seed = Data(repeating: 0x42, count: 32)
        let keys = try MasterKeys.fromSeed(seed: seed, network: .bitcoin)
        let address = try keys.receiveAddress(index: 0, network: .bitcoin)
        let text = "\(address)"
        XCTAssertTrue(text.hasPrefix("ltcmweb1"), "expected Litecoin MWEB HRP, got \(text)")
        XCTAssertTrue(mwebIsMwebAddress(address: address))
    }

    func testMwebStoreStartsEmpty() {
        let store = MwebStore()
        XCTAssertEqual(store.unspentCount(), 0)
    }

    func testCombinedBalanceOnEmptyWallet() throws {
        let mnemonic = try Mnemonic.fromString(mnemonic: "space echo position soft dizzy revise inflict damaged swallow bid thunder expend")
        let descriptor = try Descriptor.newBip84(
            secretKey: DescriptorSecretKey(networkKind: .test, mnemonic: mnemonic, password: nil),
            keychainKind: .external,
            networkKind: .test
        )
        let change = try Descriptor.newBip84(
            secretKey: DescriptorSecretKey(networkKind: .test, mnemonic: mnemonic, password: nil),
            keychainKind: .internal,
            networkKind: .test
        )
        let wallet = try Wallet(
            descriptor: descriptor,
            changeDescriptor: change,
            network: .testnet4,
            persister: try Persister.newInMemory(),
            lookahead: 25
        )
        let store = MwebStore()
        let combined = wallet.balanceCombined(store: store)
        XCTAssertEqual(combined.transparent.total.toSat(), 0)
        XCTAssertEqual(combined.mwebConfirmed.toSat(), 0)
        XCTAssertEqual(combined.mwebUntrustedPending.toSat(), 0)
    }
}
