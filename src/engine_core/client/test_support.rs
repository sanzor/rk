use super::EngineClientProvider;
use crate::engine_core::{
    EngineHandle, EngineParams, InMemoryAccountProvider, InMemoryTransactionIdRegistry,
    InMemoryTransactionLedger,
};

/// A fresh, empty engine backed by the in-memory providers, ready for a
/// test to submit instructions to — going through the real
/// [`EngineHandle`]/[`EngineClient`](super::EngineClient) queue, not
/// `Engine` directly (that's what `engine::tests` is for). Shared by
/// `tests` and `concurrent_tests`.
pub(crate) fn spawn_engine() -> impl EngineClientProvider {
    EngineHandle::start(EngineParams {
        accounts: InMemoryAccountProvider::default(),
        transactions: InMemoryTransactionLedger::default(),
        transaction_ids: InMemoryTransactionIdRegistry::default(),
    })
    .client_factory()
    .get_client()
}

/// A fresh in-memory engine pool. Each shard receives isolated account and
/// active-dispute state, while all receive a clone of one shared transaction
/// ID registry.
pub(crate) fn spawn_sharded_engine(shard_count: usize) -> impl EngineClientProvider {
    let transaction_ids = InMemoryTransactionIdRegistry::default();

    EngineHandle::start_sharded(shard_count, move || EngineParams {
        accounts: InMemoryAccountProvider::default(),
        transactions: InMemoryTransactionLedger::default(),
        transaction_ids: transaction_ids.clone(),
    })
    .client_factory()
    .get_client()
}
