//! MWEB (MimbleWimble Extension Blocks) UniFFI surface for the Litecoin BDK fork.
//!
//! Long-running sync calls block the calling thread; Swift/Kotlin callers should
//! dispatch them onto a background queue / coroutine dispatcher.

use crate::bitcoin::{Address, Amount, Network, Script, Transaction};
use crate::error::MwebError;
use crate::types::Balance;
use crate::wallet::Wallet;

use bdk_mweb::keys::MasterKeys as BdkMasterKeys;
use bdk_mweb::mweb_sync::{MwebSyncer as BdkMwebSyncer, SyncState as BdkSyncState};
use bdk_mweb::psbt_fund::FundedMwebPsbt as BdkFundedMwebPsbt;
use bdk_mweb::{
    is_mweb_address, master_keys_from_seed, parse_mweb_address, receive_address,
};
use bdk_wallet::bitcoin::key::Secp256k1;
use bdk_wallet::bitcoin::NetworkKind;
use bdk_wallet::mweb::{
    CombinedBalance as BdkCombinedBalance, MwebSpendParams, MwebStore as BdkMwebStore,
    PreparedMwebPegin as BdkPreparedMwebPegin,
};

use std::sync::{Arc, Mutex};

fn network_kind(network: Network) -> NetworkKind {
    match network {
        Network::Bitcoin => NetworkKind::Main,
        _ => NetworkKind::Test,
    }
}

/// Transparent wallet balance plus bucketed MWEB value.
#[derive(uniffi::Record)]
pub struct CombinedBalance {
    pub transparent: Balance,
    pub mweb_confirmed: Arc<Amount>,
    pub mweb_untrusted_pending: Arc<Amount>,
}

impl From<BdkCombinedBalance> for CombinedBalance {
    fn from(value: BdkCombinedBalance) -> Self {
        Self {
            transparent: Balance::from(value.transparent),
            mweb_confirmed: Arc::new(Amount::from(value.mweb_confirmed)),
            mweb_untrusted_pending: Arc::new(Amount::from(value.mweb_untrusted_pending)),
        }
    }
}

/// Master MWEB scan/spend keys derived from a BIP39 seed (or raw seed bytes).
#[derive(uniffi::Object)]
pub struct MasterKeys {
    inner: BdkMasterKeys,
}

#[uniffi::export]
impl MasterKeys {
    /// Derive MWEB master keys from raw seed bytes for `network`.
    #[uniffi::constructor]
    pub fn from_seed(seed: Vec<u8>, network: Network) -> Result<Self, MwebError> {
        let keys = master_keys_from_seed(&seed, network)?;
        Ok(Self { inner: keys })
    }

    /// Derive a receive stealth address at `index`.
    pub fn receive_address(&self, index: u32, network: Network) -> Result<Arc<Address>, MwebError> {
        let secp = Secp256k1::new();
        let addr = receive_address(&self.inner, index, network_kind(network), &secp)?;
        Ok(Arc::new(Address::from(addr)))
    }
}

/// Parallel MWEB coin store owned beside a wallet (not part of Wallet ChangeSet).
#[derive(uniffi::Object)]
pub struct MwebStore {
    inner: Mutex<BdkMwebStore>,
}

#[uniffi::export]
impl MwebStore {
    /// Empty in-memory store.
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(BdkMwebStore::new()),
        }
    }

    /// Number of unspent MWEB coins currently in the store.
    pub fn unspent_count(&self) -> u64 {
        self.inner.lock().unwrap().db().unspent_vec().len() as u64
    }

    /// Clear inclusion heights for coins at or above `height` (reorg disconnect).
    pub fn disconnect_from(&self, height: u32) {
        self.inner.lock().unwrap().disconnect_from(height);
    }

    /// Load from a SQLite file path (creates tables if missing).
    #[uniffi::constructor]
    pub fn load_sqlite(path: String) -> Result<Self, MwebError> {
        let mut conn = bdk_wallet::rusqlite::Connection::open(&path)
            .map_err(|e| MwebError::Sqlite {
                error_message: e.to_string(),
            })?;
        let tx = conn.transaction().map_err(|e| MwebError::Sqlite {
            error_message: e.to_string(),
        })?;
        let store = BdkMwebStore::load_sqlite(&tx).map_err(|e| MwebError::Sqlite {
            error_message: e.to_string(),
        })?;
        tx.commit().map_err(|e| MwebError::Sqlite {
            error_message: e.to_string(),
        })?;
        Ok(Self {
            inner: Mutex::new(store),
        })
    }

    /// Persist staged mutations into a SQLite file path.
    pub fn persist_sqlite(&self, path: String) -> Result<(), MwebError> {
        let mut store = self.inner.lock().unwrap();
        let mut conn = bdk_wallet::rusqlite::Connection::open(&path)
            .map_err(|e| MwebError::Sqlite {
                error_message: e.to_string(),
            })?;
        let tx = conn.transaction().map_err(|e| MwebError::Sqlite {
            error_message: e.to_string(),
        })?;
        bdk_mweb::ChangeSet::init_sqlite_tables(&tx).map_err(|e| MwebError::Sqlite {
            error_message: e.to_string(),
        })?;
        store.persist_sqlite(&tx).map_err(|e| MwebError::Sqlite {
            error_message: e.to_string(),
        })?;
        tx.commit().map_err(|e| MwebError::Sqlite {
            error_message: e.to_string(),
        })?;
        Ok(())
    }
}

impl MwebStore {
    pub(crate) fn with_db<R>(&self, f: impl FnOnce(&bdk_mweb::MwebCoinDatabase) -> R) -> R {
        let store = self.inner.lock().unwrap();
        f(store.db())
    }

    pub(crate) fn with_db_mut<R>(
        &self,
        f: impl FnOnce(&mut bdk_mweb::MwebCoinDatabase) -> R,
    ) -> R {
        let mut store = self.inner.lock().unwrap();
        f(store.db_mut())
    }
}

/// Result of preparing a peg-in: transparent PSBT + owned MWEB outputs to insert after maturity.
#[derive(uniffi::Object)]
pub struct PreparedMwebPegin {
    inner: Mutex<BdkPreparedMwebPegin>,
}

#[uniffi::export]
impl PreparedMwebPegin {
    /// Serialize the transparent peg-in PSBT (with MWEB maps) as base64.
    pub fn psbt_base64(&self) -> String {
        self.inner.lock().unwrap().psbt.to_string()
    }

    /// Peg-in kernel id (32 bytes = v9 witness program).
    pub fn kernel_id(&self) -> Vec<u8> {
        self.inner.lock().unwrap().kernel_id.to_vec()
    }

    /// Transparent v9 output value.
    pub fn pegin_amount(&self) -> Arc<Amount> {
        Arc::new(Amount::from(self.inner.lock().unwrap().pegin_amount))
    }

    /// Number of owned stealth outputs produced for insertion after maturity.
    pub fn output_count(&self) -> u64 {
        self.inner.lock().unwrap().outputs.len() as u64
    }

    /// Insert the prepared outputs into `store` (call after peg-in maturity).
    pub fn insert_outputs_into_store(&self, store: &MwebStore) {
        let prepared = self.inner.lock().unwrap();
        store.with_db_mut(|db| {
            for coin in &prepared.outputs {
                db.insert(coin.clone());
            }
        });
    }
}

/// Funded MWEB PSBT (maps only) awaiting signing/extraction.
#[derive(uniffi::Object)]
pub struct FundedMwebPsbt {
    inner: Mutex<BdkFundedMwebPsbt>,
}

/// Blocking LIP-0006 MWEB sync helper.
///
/// Call from a background thread; this blocks while talking to peers.
#[derive(uniffi::Object)]
pub struct MwebSyncer {
    inner: Mutex<BdkMwebSyncer>,
}

#[uniffi::export]
impl MwebSyncer {
    /// Create a syncer with defaults, optionally overriding UTXO batch size.
    #[uniffi::constructor(default(batch_size = None))]
    pub fn new(batch_size: Option<u16>) -> Self {
        let mut syncer = BdkMwebSyncer::new();
        if let Some(size) = batch_size {
            syncer.batch_size = size;
        }
        Self {
            inner: Mutex::new(syncer),
        }
    }

    /// Configured UTXO batch size (clamped at use time).
    pub fn batch_size(&self) -> u16 {
        self.inner.lock().unwrap().batch_size
    }
}

/// Resumable sync cursor (leafset + download position).
#[derive(uniffi::Object)]
pub struct SyncState {
    #[allow(dead_code)] // exposed for future differential-sync wrappers
    inner: Mutex<BdkSyncState>,
}

#[uniffi::export]
impl SyncState {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(BdkSyncState::new()),
        }
    }
}

/// Parse an MWEB stealth address string for `network`.
#[uniffi::export]
pub fn mweb_parse_address(address: String, network: Network) -> Result<Arc<Address>, MwebError> {
    let addr = parse_mweb_address(&address, network)?;
    Ok(Arc::new(Address::from(addr)))
}

/// Return true if `address` is an MWEB stealth address.
#[uniffi::export]
pub fn mweb_is_mweb_address(address: Arc<Address>) -> bool {
    is_mweb_address(&address.0)
}

/// Wallet extensions for combined balance and MWEB transaction flows.
#[uniffi::export]
impl Wallet {
    /// Transparent balance plus MWEB buckets at the wallet tip.
    pub fn balance_combined(&self, store: &MwebStore) -> CombinedBalance {
        let wallet = self.get_wallet();
        store.with_db(|db| CombinedBalance::from(wallet.balance_combined(db)))
    }

    /// Prepare a peg-in: fund+sign MWEB maps and assemble the transparent v9 PSBT.
    ///
    /// Caller must sign transparent inputs on the returned PSBT, extract/broadcast,
    /// then call [`PreparedMwebPegin::insert_outputs_into_store`] after maturity.
    pub fn prepare_mweb_pegin(
        &self,
        keys: &MasterKeys,
        receive_index: u32,
        pegin_amount: Arc<Amount>,
        mweb_fee: Arc<Amount>,
        transparent_fee: Arc<Amount>,
    ) -> Result<Arc<PreparedMwebPegin>, MwebError> {
        let secp = Secp256k1::new();
        let mut wallet = self.get_wallet();
        let prepared = wallet.prepare_mweb_pegin(
            &keys.inner,
            receive_index,
            pegin_amount.0,
            mweb_fee.0,
            transparent_fee.0,
            &secp,
        )?;
        Ok(Arc::new(PreparedMwebPegin {
            inner: Mutex::new(prepared),
        }))
    }

    /// Fund an MWEB→MWEB send from spendable coins in `store`.
    pub fn fund_mweb_send(
        &self,
        store: &MwebStore,
        keys: &MasterKeys,
        recipient: Arc<Address>,
        amount: Arc<Amount>,
        fee: Arc<Amount>,
        change_index: u32,
    ) -> Result<Arc<FundedMwebPsbt>, MwebError> {
        let secp = Secp256k1::new();
        let wallet = self.get_wallet();
        let params = MwebSpendParams::new(amount.0, fee.0, change_index);
        let funded = store.with_db(|db| {
            wallet.fund_mweb_send(db, &keys.inner, recipient.0.clone(), params, &secp)
        })?;
        Ok(Arc::new(FundedMwebPsbt {
            inner: Mutex::new(funded),
        }))
    }

    /// Fund an MWEB→LTC peg-out to `script_pubkey` from spendable coins in `store`.
    pub fn fund_mweb_pegout(
        &self,
        store: &MwebStore,
        keys: &MasterKeys,
        script_pubkey: Arc<Script>,
        amount: Arc<Amount>,
        fee: Arc<Amount>,
        change_index: u32,
    ) -> Result<Arc<FundedMwebPsbt>, MwebError> {
        let secp = Secp256k1::new();
        let wallet = self.get_wallet();
        let params = MwebSpendParams::new(amount.0, fee.0, change_index);
        let funded = store.with_db(|db| {
            wallet.fund_mweb_pegout(
                db,
                &keys.inner,
                script_pubkey.0.clone(),
                params,
                &secp,
            )
        })?;
        Ok(Arc::new(FundedMwebPsbt {
            inner: Mutex::new(funded),
        }))
    }

    /// Sign a funded MWEB PSBT and extract the network transaction.
    ///
    /// Returns the extracted transaction. Change (if any) is inserted into `store`.
    pub fn sign_and_extract_funded_mweb(
        &self,
        funded: &FundedMwebPsbt,
        keys: &MasterKeys,
        store: &MwebStore,
    ) -> Result<Arc<Transaction>, MwebError> {
        let secp = Secp256k1::new();
        let wallet = self.get_wallet();
        let mut funded_inner = funded.inner.lock().unwrap();
        let (tx, change) =
            wallet.sign_and_extract_funded_mweb(&mut funded_inner, &keys.inner, &secp)?;
        if let Some(change_coin) = change {
            store.with_db_mut(|db| {
                db.insert(change_coin);
            });
        }
        Ok(Arc::new(Transaction::from(tx)))
    }
}
