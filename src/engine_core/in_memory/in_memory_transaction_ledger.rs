use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::domain::{ClientId, TransactionId};
use crate::engine_core::transaction_ledger::TransactionLedger;
use crate::engine_core::transaction_record::DisputableTransaction;

/// The default [`TransactionLedger`]: disputable deposits live in a
/// `HashMap` for the lifetime of the process, with no further
/// persistence.
#[derive(Debug, Default)]
pub struct InMemoryTransactionLedger {
    deposits: HashMap<TransactionId, DisputableTransaction>,
}

#[async_trait::async_trait]
impl TransactionLedger for InMemoryTransactionLedger {
    async fn record_deposit(&mut self, tx_id: TransactionId, client_id: ClientId, amount: Decimal) {
        self.deposits.insert(
            tx_id,
            DisputableTransaction {
                client_id,
                amount,
                disputed: false,
            },
        );
    }

    async fn lookup_disputable(&mut self, tx_id: TransactionId) -> Option<DisputableTransaction> {
        self.deposits.get(&tx_id).copied()
    }

    async fn mark_disputed(&mut self, tx_id: TransactionId) {
        if let Some(deposit) = self.deposits.get_mut(&tx_id) {
            deposit.disputed = true;
        }
    }

    async fn remove(&mut self, tx_id: TransactionId) {
        self.deposits.remove(&tx_id);
    }
}
