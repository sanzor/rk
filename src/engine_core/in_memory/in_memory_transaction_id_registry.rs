use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::domain::TransactionId;
use crate::engine_core::transaction_id_registry::TransactionIdRegistry;

/// Process-local, shareable transaction-ID registry.
///
/// Clones share one `HashSet`; the mutex covers only the atomic membership
/// check and insertion. Engine code drops that short-lived lock before doing
/// any asynchronous work.
#[derive(Debug, Clone, Default)]
pub struct InMemoryTransactionIdRegistry {
    transaction_ids: Arc<Mutex<HashSet<TransactionId>>>,
}

impl TransactionIdRegistry for InMemoryTransactionIdRegistry {
    fn reserve(&self, tx_id: TransactionId) -> bool {
        let mut transaction_ids = self
            .transaction_ids
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        transaction_ids.insert(tx_id)
    }
}
