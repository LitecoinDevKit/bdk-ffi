package org.litecoindevkit

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class MwebSmokeTest {
    @Test
    fun masterKeysDeriveMwebAddress() {
        val seed = ByteArray(32) { 0x42 }
        val keys = MasterKeys.fromSeed(seed, Network.BITCOIN)
        val address = keys.receiveAddress(0u, Network.BITCOIN)
        assertTrue(address.toString().startsWith("ltcmweb1"))
        assertTrue(mwebIsMwebAddress(address))
    }

    @Test
    fun combinedBalanceOnEmptyWallet() {
        val mnemonic = Mnemonic.fromString(
            "space echo position soft dizzy revise inflict damaged swallow bid thunder expend"
        )
        val secret = DescriptorSecretKey(NetworkKind.TEST, mnemonic, null)
        val descriptor = Descriptor.newBip84(secret, KeychainKind.EXTERNAL, NetworkKind.TEST)
        val change = Descriptor.newBip84(secret, KeychainKind.INTERNAL, NetworkKind.TEST)
        val wallet = Wallet(
            descriptor,
            change,
            Network.TESTNET4,
            Persister.newInMemory(),
            25u,
        )
        val store = MwebStore()
        val combined = wallet.balanceCombined(store)
        assertEquals(0uL, combined.transparent.total.toSat())
        assertEquals(0uL, combined.mwebConfirmed.toSat())
        assertEquals(0uL, combined.mwebUntrustedPending.toSat())
    }
}
