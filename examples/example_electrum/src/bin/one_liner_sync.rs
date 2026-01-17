//! Example of a wallet sync using BdkElectrumClient.
//!
//! This example demonstrates how to:
//! 1. Create a wallet (IndexedTxGraph with KeychainTxOutIndex).
//! 2. Create an Electrum client.
//! 3. Perform a full scan to discover used scripts.
//!
//! Note: This example requires an actual Electrum server URL to run successfully.
//! By default it tries to connect to a public testnet server.

use bdk_chain::{
    bitcoin::{secp256k1::Secp256k1, BlockHash},
    keychain_txout::KeychainTxOutIndex,
    local_chain::LocalChain,
    miniscript::Descriptor,
    spk_client::FullScanRequest,
    IndexedTxGraph,
};
use bdk_electrum::{
    electrum_client::{self},
    BdkElectrumClient,
};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum MyKeychain {
    External,
    Internal,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const ELECTRUM_URL: &str = "ssl://electrum.blockstream.info:60002"; // Testnet

    // 1. Setup Wallet: IndexedTxGraph enclosing KeychainTxOutIndex
    // We use a LocalChain to track the chain tip (testnet genesis hash defaulting)
    let (mut chain, _) = LocalChain::from_genesis(BlockHash::from_str(
        "000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943",
    )?);

    let mut graph = IndexedTxGraph::new(KeychainTxOutIndex::<MyKeychain>::new(20, true));

    // Add descriptors
    let secp = Secp256k1::new();
    let (external_descriptor, _) = Descriptor::parse_descriptor(&secp, "tr([73c5da0a/86'/1'/0']tpubDCDkM3bAi3d7KqW8G9w8V9w8V9w8V9w8V9w8V9w8V9w8V9w8V9w8V9w8V9w8V-testnet/0/*)")?;
    let (internal_descriptor, _) = Descriptor::parse_descriptor(&secp, "tr([73c5da0a/86'/1'/0']tpubDCDkM3bAi3d7KqW8G9w8V9w8V9w8V9w8V9w8V9w8V9w8V9w8V9w8V9w8V9w8V-testnet/1/*)")?;

    graph.index.insert_descriptor(MyKeychain::External, external_descriptor)?;
    graph.index.insert_descriptor(MyKeychain::Internal, internal_descriptor)?;

    println!("Wallet initialized.");

    // 2. Setup Electrum Client
    let electrum_client = electrum_client::Client::new(ELECTRUM_URL)?;
    // Wrap it in BdkElectrumClient
    let bdk_client = BdkElectrumClient::new(electrum_client);

    // 3. Sync
    println!("Starting full scan...");

    // Construct request
    let request = FullScanRequest::builder()
        .chain_tip(chain.tip())
        .spks_for_keychain(MyKeychain::External, graph.index.unbounded_spk_iter(MyKeychain::External).unwrap())
        .spks_for_keychain(MyKeychain::Internal, graph.index.unbounded_spk_iter(MyKeychain::Internal).unwrap());

    // Perform scan
    let update = bdk_client.full_scan(request, 10, 10, true)?;

    // Apply updates
    if let Some(chain_update) = update.chain_update {
        chain.apply_update(chain_update)?;
    }
    let _ = graph.apply_update(update.tx_update);

    println!("Sync finished!");
    println!("New tip: {:?}", chain.tip());
    println!("Found transactions: {}", graph.graph().full_txs().count());

    Ok(())
}
