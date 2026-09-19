use crate::{
    block::{SolizoneBlock, SolizoneBlockHeader},
    execution::{
        receipt::ExecutionReceipt, receipts_root::compute_receipts_root,
        transaction::SolizoneTransaction, transactions_root::compute_transactions_root,
    },
};

use revm::primitives::B256;

pub fn build_block_header(
    version: u16,
    chain_id: u64,
    height: u64,
    parent_hash: B256,
    timestamp: u64,
    state_root: B256,
    transactions: &[SolizoneTransaction],
    receipts: &[ExecutionReceipt],
    gas_limit: u64,
) -> SolizoneBlockHeader {
    let transactions_root = compute_transactions_root(transactions);

    let receipts_root = compute_receipts_root(receipts);

    let gas_used = receipts.iter().map(|r| r.gas_used).sum();

    SolizoneBlockHeader {
        version,
        chain_id,
        height,
        parent_hash,
        timestamp,
        state_root,
        transactions_root,
        receipts_root,
        gas_limit,
        gas_used,
    }
}

pub fn build_block(
    version: u16,
    chain_id: u64,
    height: u64,
    parent_hash: B256,
    timestamp: u64,
    state_root: B256,
    transactions: &[SolizoneTransaction],
    receipts: &[ExecutionReceipt],
    gas_limit: u64,
) -> SolizoneBlock {
    let header = build_block_header(
        version,
        chain_id,
        height,
        parent_hash,
        timestamp,
        state_root,
        transactions,
        receipts,
        gas_limit,
    );

    SolizoneBlock {
        header,
        transactions: transactions.to_vec(),
    }
}
