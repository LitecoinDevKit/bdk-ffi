mod bitcoin;
mod descriptor;
mod electrum;
mod error;
mod esplora;
mod keys;
mod macros;
mod mweb;
mod signer;
mod store;
mod tx_builder;
mod types;
mod wallet;

#[cfg(test)]
mod tests;

use crate::bitcoin::OutPoint;

uniffi::setup_scaffolding!("bdk");
