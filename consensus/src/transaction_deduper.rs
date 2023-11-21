// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::txn_hash_and_authenticator_deduper::TxnHashAndAuthenticatorDeduper;
use aptos_logger::info;
use aptos_types::{
    on_chain_config::TransactionDeduperType, transaction::DeprecatedSignedUserTransaction,
};
use std::sync::Arc;

/// Interface to dedup transactions. The dedup filters duplicate transactions within a block.
pub trait TransactionDeduper: Send + Sync {
    fn dedup(
        &self,
        txns: Vec<DeprecatedSignedUserTransaction>,
    ) -> Vec<DeprecatedSignedUserTransaction>;
}

/// No Op Deduper to maintain backward compatibility
struct NoOpDeduper {}

impl TransactionDeduper for NoOpDeduper {
    fn dedup(
        &self,
        txns: Vec<DeprecatedSignedUserTransaction>,
    ) -> Vec<DeprecatedSignedUserTransaction> {
        txns
    }
}

pub fn create_transaction_deduper(
    deduper_type: TransactionDeduperType,
) -> Arc<dyn TransactionDeduper> {
    match deduper_type {
        TransactionDeduperType::NoDedup => Arc::new(NoOpDeduper {}),
        TransactionDeduperType::TxnHashAndAuthenticatorV1 => {
            info!("Using simple hash set transaction deduper");
            Arc::new(TxnHashAndAuthenticatorDeduper::new())
        },
    }
}
