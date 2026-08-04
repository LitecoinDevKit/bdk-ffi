//! MWEB (MimbleWimble Extension Blocks) UniFFI surface for the Litecoin BDK fork.
//!
//! Long-running sync calls block the calling thread; Swift/Kotlin callers should
//! dispatch them onto a background queue / coroutine dispatcher.

use crate::bitcoin::{Address, Amount, Network, Script, Transaction};
use crate::error::MwebError;
use crate::types::Balance;
use crate::wallet::Wallet;

use bdk_mweb::keys::{MasterKeyScheme as BdkMasterKeyScheme, MasterKeys as BdkMasterKeys};
use bdk_mweb::lip0006_tcp::{BroadcastAck, TcpMwebPeer};
use bdk_mweb::mweb_sync::{
    FixedHeaderProvider, MwebSyncer as BdkMwebSyncer, PeerPool, ReadyNotifier,
    SyncProgress as BdkSyncProgress, SyncState as BdkSyncState,
};
use bdk_mweb::psbt_fund::FundedMwebPsbt as BdkFundedMwebPsbt;
use bdk_mweb::{
    is_mweb_address, master_keys_from_seed, parse_mweb_address, receive_address,
    MWEB_PEGIN_MATURITY,
};
use bdk_wallet::bitcoin::key::Secp256k1;
use bdk_wallet::bitcoin::NetworkKind;
use bdk_wallet::mweb::{
    CombinedBalance as BdkCombinedBalance, MwebScanContext, MwebSpendParams,
    MwebStore as BdkMwebStore, MwebSyncDrivers, PreparedMwebPegin as BdkPreparedMwebPegin,
};
use bdk_wallet::serde_json;

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};

/// MWEB address-book size used for ownership scanning during sync.
///
/// Ownership lookup is a single map probe regardless of book size, so this is
/// sized for recovery: coins received at any index below it are found on
/// restore.
const MWEB_GAP_LIMIT: u32 = 1000;

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

/// Which BIP32 layout derives the MWEB master scan/spend keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Enum)]
pub enum MasterKeyScheme {
    /// Litecoin Core 0.21: `m/0'/100'/{0,1}'` (default; what litecoind wallets use).
    #[default]
    LitecoinCore,
    /// LIP-0004 text: `m/1/0/100'` and `m/1/0/101'`.
    Lip0004,
    /// mwebd / Nexus (BIP43 purpose 1000, coin 2, account 0): `m/1000'/2'/0'/{0,1}'`.
    Mwebd,
}

impl From<MasterKeyScheme> for BdkMasterKeyScheme {
    fn from(value: MasterKeyScheme) -> Self {
        match value {
            MasterKeyScheme::LitecoinCore => BdkMasterKeyScheme::LitecoinCore,
            MasterKeyScheme::Lip0004 => BdkMasterKeyScheme::Lip0004,
            MasterKeyScheme::Mwebd => BdkMasterKeyScheme::Mwebd,
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
    /// Derive MWEB master keys from raw seed bytes for `network`
    /// (LitecoinCore scheme).
    #[uniffi::constructor]
    pub fn from_seed(seed: Vec<u8>, network: Network) -> Result<Self, MwebError> {
        let keys = master_keys_from_seed(&seed, network)?;
        Ok(Self { inner: keys })
    }

    /// Derive MWEB master keys from raw seed bytes using an explicit
    /// derivation `scheme`.
    #[uniffi::constructor]
    pub fn from_seed_with_scheme(
        seed: Vec<u8>,
        scheme: MasterKeyScheme,
        network: Network,
    ) -> Result<Self, MwebError> {
        let secp = Secp256k1::new();
        let keys = BdkMasterKeys::from_seed(&seed, network, scheme.into(), &secp)?;
        Ok(Self { inner: keys })
    }

    /// Derive MWEB master keys from a BIP39 mnemonic (optionally with a
    /// passphrase), so MWEB keys come from the same phrase as the transparent
    /// descriptors.
    #[uniffi::constructor(default(passphrase = None, scheme = None))]
    pub fn from_mnemonic(
        mnemonic: Arc<crate::keys::Mnemonic>,
        passphrase: Option<String>,
        scheme: Option<MasterKeyScheme>,
        network: Network,
    ) -> Result<Self, MwebError> {
        let seed = mnemonic.to_seed(passphrase);
        Self::from_seed_with_scheme(seed, scheme.unwrap_or_default(), network)
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

    /// Sum of mature, spendable MWEB coin value at `tip_height` (litoshis).
    ///
    /// Stricter than confirmed balance: peg-in outputs only count after
    /// [`MWEB_PEGIN_MATURITY`] (6) blocks.
    pub fn spendable_balance(&self, tip_height: u32) -> u64 {
        self.inner
            .lock()
            .unwrap()
            .db()
            .unspent_spendable(tip_height, MWEB_PEGIN_MATURITY)
            .iter()
            .fold(0u64, |acc, c| acc.saturating_add(c.amount))
    }

    /// Mark coins spent by output id (32 bytes each) after a broadcast, so the
    /// local balance drops immediately instead of waiting for the next sync.
    ///
    /// Returns the number of coins that were found and marked.
    pub fn mark_spent(&self, output_ids: Vec<Vec<u8>>) -> u64 {
        let mut store = self.inner.lock().unwrap();
        let mut marked = 0u64;
        for id in output_ids {
            let Ok(id) = <[u8; 32]>::try_from(id.as_slice()) else {
                continue;
            };
            if store.db_mut().mark_spent(&id) {
                marked += 1;
            }
        }
        marked
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

#[uniffi::export]
impl FundedMwebPsbt {
    /// Output ids (32 bytes each) of the coins this transaction spends.
    ///
    /// Pass these to [`MwebStore::mark_spent`] after a successful broadcast to
    /// mirror the spend locally before the next sync confirms it.
    pub fn spent_coin_ids(&self) -> Vec<Vec<u8>> {
        self.inner
            .lock()
            .unwrap()
            .spent_coins
            .iter()
            .map(|c| c.output_id.to_vec())
            .collect()
    }

    /// Kernel fee this transaction pays (litoshis).
    pub fn fee(&self) -> u64 {
        self.inner.lock().unwrap().fee
    }
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
///
/// Persist with [`SyncState::to_json`] beside the MWEB SQLite store and
/// restore with [`SyncState::from_json`] so later syncs only download new
/// leaves instead of the full UTXO set.
#[derive(uniffi::Object)]
pub struct SyncState {
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

    /// Restore a sync cursor persisted with [`SyncState::to_json`].
    #[uniffi::constructor]
    pub fn from_json(json: String) -> Result<Self, MwebError> {
        let state: BdkSyncState =
            serde_json::from_str(&json).map_err(|e| MwebError::Mweb {
                error_message: format!("sync state json: {e}"),
            })?;
        Ok(Self {
            inner: Mutex::new(state),
        })
    }

    /// Serialize the sync cursor for persistence across app launches.
    pub fn to_json(&self) -> Result<String, MwebError> {
        serde_json::to_string(&*self.inner.lock().unwrap()).map_err(|e| MwebError::Mweb {
            error_message: format!("sync state json: {e}"),
        })
    }

    /// Height of the last verified tip, if any.
    pub fn tip_height(&self) -> Option<u32> {
        self.inner.lock().unwrap().tip_height
    }

    /// Drop the leafset snapshot so the next pass re-downloads (reorg recovery).
    pub fn invalidate_leafset(&self) {
        self.inner.lock().unwrap().invalidate_leafset();
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

/// Discover public LIP-0006 (MWEB light-client) peers via Litecoin's DNS seeds.
///
/// Blocking (DNS + a short TCP handshake per candidate); call from a background
/// thread. Results are cached for 15 minutes. Returns `host:port` strings ready
/// for [`MwebSyncManager::sync_at_tip`] / [`mweb_broadcast`]; empty when
/// nothing answered.
#[uniffi::export]
pub fn mweb_discover_peers(network: Network) -> Vec<String> {
    bdk_mweb::discovery::discover_mweb_peers(network)
        .into_iter()
        .map(|a| a.to_string())
        .collect()
}

/// Outcome of an MWEB P2P broadcast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MwebBroadcastResult {
    /// A peer re-announced the tx back to us: accepted into its mempool.
    Confirmed,
    /// Sent to a peer; acceptance not confirmed before the deadline.
    Sent,
}

/// Broadcast a transaction carrying MWEB data (peg-in, peg-out, MWEB send)
/// over the LIP-0006 P2P protocol.
///
/// Electrum servers cannot relay MWEB transactions, so sends and peg-outs must
/// take this path. Tries each peer in turn (`host:port` strings, e.g. from
/// [`mweb_discover_peers`]); blocking — call from a background thread.
#[uniffi::export]
pub fn mweb_broadcast(
    transaction: Arc<Transaction>,
    peers: Vec<String>,
    network: Network,
) -> Result<MwebBroadcastResult, MwebError> {
    let tx = bdk_wallet::bitcoin::Transaction::from(transaction.as_ref());
    let addrs = resolve_peers(&peers);
    if addrs.is_empty() {
        return Err(MwebError::NoPeers {
            error_message: "no MWEB P2P peers given or resolvable".to_string(),
        });
    }
    let mut last_err = String::from("no MWEB peers reachable");
    for addr in addrs {
        match TcpMwebPeer::connect(addr, network) {
            Ok(mut peer) => match peer.broadcast_tx(&tx) {
                Ok(BroadcastAck::Confirmed) => return Ok(MwebBroadcastResult::Confirmed),
                Ok(BroadcastAck::Sent) => return Ok(MwebBroadcastResult::Sent),
                Err(e) => last_err = format!("{addr}: {e}"),
            },
            Err(e) => last_err = format!("{addr}: {e}"),
        }
    }
    Err(MwebError::Mweb {
        error_message: format!("P2P broadcast failed: {last_err}"),
    })
}

fn resolve_peers(peers: &[String]) -> Vec<SocketAddr> {
    let mut out = Vec::new();
    for p in peers {
        if let Ok(iter) = p.to_socket_addrs() {
            for addr in iter {
                if !out.contains(&addr) {
                    out.push(addr);
                }
            }
        }
    }
    out
}

/// Live download progress of a sync pass (readable from another thread).
#[derive(uniffi::Record)]
pub struct MwebSyncProgress {
    /// Added leaves fetched so far in the current pass.
    pub fetched: u64,
    /// Total added leaves the current pass will fetch.
    pub total: u64,
}

/// Result of one [`MwebSyncManager::sync_at_tip`] pass.
#[derive(uniffi::Record)]
pub struct MwebSyncReport {
    /// Transparent tip height the pass verified against.
    pub tip_height: u32,
    /// Number of full UTXO outputs downloaded.
    pub downloaded: u64,
    /// Number of newly found owned coins.
    pub found: u64,
    /// Number of local coins marked spent (gone from the UTXO set).
    pub spent: u64,
}

/// Runnable LIP-0006 MWEB sync: what ltc-wallet-mac's runtime does natively,
/// exposed for mobile.
///
/// One differential pass downloads the leafset diff since the last
/// [`SyncState`], fetches new UTXOs in batches, verifies them against the
/// chain-committed MWEB header, and scans for coins owned by [`MasterKeys`].
///
/// Blocking — dispatch onto a background queue / coroutine dispatcher. While a
/// pass runs, [`MwebSyncManager::progress`] can be polled from another thread.
#[derive(uniffi::Object)]
pub struct MwebSyncManager {
    progress: Arc<BdkSyncProgress>,
}

#[uniffi::export]
impl MwebSyncManager {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            progress: Arc::new(BdkSyncProgress::default()),
        }
    }

    /// `(fetched, total)` snapshot of the current pass's UTXO download.
    pub fn progress(&self) -> MwebSyncProgress {
        let (fetched, total) = self.progress.snapshot();
        MwebSyncProgress { fetched, total }
    }

    /// One differential sync pass at `wallet`'s current tip.
    ///
    /// - `peers` are `host:port` strings (from [`mweb_discover_peers`] or the
    ///   caller's own node). Peers are rotated on failure; a rate-limiting peer
    ///   surfaces as [`MwebError::Throttled`] — back off instead of retrying
    ///   immediately.
    /// - `state` carries the leafset across passes: persist it (and the store)
    ///   after each pass so the next launch only downloads new leaves.
    /// - Found coins land in `store`; spent coins are marked.
    ///
    /// Run an Electrum/Esplora sync first so the wallet tip is current.
    pub fn sync_at_tip(
        &self,
        wallet: &Wallet,
        store: &MwebStore,
        keys: &MasterKeys,
        state: &SyncState,
        peers: Vec<String>,
    ) -> Result<MwebSyncReport, MwebError> {
        let network = wallet.network();
        let addrs = {
            let mut addrs = resolve_peers(&peers);
            if addrs.is_empty() {
                addrs = bdk_mweb::discovery::discover_mweb_peers(network);
            }
            addrs
        };
        if addrs.is_empty() {
            return Err(MwebError::NoPeers {
                error_message: "no MWEB peers given, and none found via DNS seeds".to_string(),
            });
        }

        // Read the tip and the previous tip's hash-on-current-chain, then drop
        // the wallet lock before the (long) network phase.
        let (tip_hash, tip_height, prev) = {
            let inner_wallet = wallet.get_wallet();
            let cp = inner_wallet.latest_checkpoint();
            let prev = state
                .inner
                .lock()
                .unwrap()
                .tip_height
                .and_then(|h| cp.get(h).map(|p| (h, p.hash())));
            (cp.hash(), cp.height(), prev)
        };

        let syncer = BdkMwebSyncer {
            progress: Some(self.progress.clone()),
            ..BdkMwebSyncer::tip_only()
        };
        let mut hashes = BTreeMap::new();
        if let Some((prev_height, prev_hash)) = prev {
            hashes.insert(prev_height, prev_hash);
        }
        hashes.insert(tip_height, tip_hash);
        let headers = FixedHeaderProvider {
            tip_hash,
            tip_height,
            hashes,
        };
        let mut notifier = ReadyNotifier { tip_height };

        let secp = Secp256k1::new();
        let book = bdk_mweb::AddressBook::from_keys(&keys.inner, MWEB_GAP_LIMIT, &secp)?;

        let mut store_guard = store.inner.lock().unwrap();
        let mut state_guard = state.inner.lock().unwrap();
        let mut pool = PeerPool::new(addrs);
        let result = pool.with_failover(network, |peer| {
            store_guard.sync_differential(
                MwebSyncDrivers {
                    syncer: &syncer,
                    headers: &headers,
                    notifier: &mut notifier,
                    state: &mut state_guard,
                },
                peer,
                MwebScanContext {
                    keys: &keys.inner,
                    book: &book,
                    secp: &secp,
                },
            )
        })?;

        Ok(MwebSyncReport {
            tip_height,
            downloaded: result.downloaded as u64,
            found: result.found.len() as u64,
            spent: result.spent.len() as u64,
        })
    }
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
        if let Some(mut change_coin) = change {
            // Unconfirmed until the next sync dates it on-chain.
            change_coin.block_height = None;
            store.with_db_mut(|db| {
                db.insert(change_coin);
            });
        }
        Ok(Arc::new(Transaction::from(tx)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Mnemonic;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn sync_state_json_round_trips() {
        let state = SyncState::new();
        {
            let mut inner = state.inner.lock().unwrap();
            inner.tip_height = Some(1234);
            inner.leafset = vec![0b1010_0000];
        }
        let json = state.to_json().unwrap();
        let restored = SyncState::from_json(json).unwrap();
        assert_eq!(restored.tip_height(), Some(1234));
        assert_eq!(
            *restored.inner.lock().unwrap(),
            *state.inner.lock().unwrap()
        );
    }

    #[test]
    fn from_json_rejects_garbage() {
        assert!(SyncState::from_json("not json".to_string()).is_err());
    }

    /// Mnemonic → seed → LitecoinCore-scheme keys must match wallet-core's
    /// derivation (master xprv → m/0'/100'/{0,1}'), and differ across schemes.
    #[test]
    fn from_mnemonic_matches_seed_derivation_and_schemes_differ() {
        let mnemonic = Arc::new(Mnemonic::from_string(TEST_MNEMONIC.to_string()).unwrap());
        let network = Network::Testnet4;

        let via_mnemonic =
            MasterKeys::from_mnemonic(mnemonic.clone(), None, None, network).unwrap();
        let via_seed = MasterKeys::from_seed(mnemonic.to_seed(None), network).unwrap();
        let secp = Secp256k1::new();
        assert_eq!(
            via_mnemonic.inner.address(0, NetworkKind::Test, &secp).unwrap(),
            via_seed.inner.address(0, NetworkKind::Test, &secp).unwrap(),
        );

        let mwebd = MasterKeys::from_mnemonic(
            mnemonic.clone(),
            None,
            Some(MasterKeyScheme::Mwebd),
            network,
        )
        .unwrap();
        assert_ne!(
            mwebd.inner.address(0, NetworkKind::Test, &secp).unwrap(),
            via_seed.inner.address(0, NetworkKind::Test, &secp).unwrap(),
        );

        let with_passphrase =
            MasterKeys::from_mnemonic(mnemonic, Some("hunter2".to_string()), None, network)
                .unwrap();
        assert_ne!(
            with_passphrase.inner.address(0, NetworkKind::Test, &secp).unwrap(),
            via_seed.inner.address(0, NetworkKind::Test, &secp).unwrap(),
        );
    }

    #[test]
    fn resolve_peers_handles_bad_and_duplicate_entries() {
        let addrs = resolve_peers(&[
            "127.0.0.1:19335".to_string(),
            "127.0.0.1:19335".to_string(),
            "not a peer".to_string(),
        ]);
        assert_eq!(addrs.len(), 1);
        assert_eq!(addrs[0].port(), 19335);
    }

    #[test]
    fn mark_spent_ignores_malformed_ids() {
        let store = MwebStore::new();
        assert_eq!(store.mark_spent(vec![vec![0u8; 5], vec![1u8; 32]]), 0);
    }
}
