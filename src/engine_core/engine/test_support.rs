use super::engine::Engine;
use crate::engine_core::{
    InMemoryAccountProvider, InMemoryTransactionIdRegistry, InMemoryTransactionLedger,
};

/// A fresh [`Engine`], backed by the in-memory providers, for a test to
/// call `apply` on directly — no queue, no `EngineHandle`, just the
/// business logic in isolation. Shared by `tests` and `property_tests`.
pub(crate) fn new_engine()
-> Engine<InMemoryAccountProvider, InMemoryTransactionLedger, InMemoryTransactionIdRegistry> {
    Engine::new(
        InMemoryAccountProvider::default(),
        InMemoryTransactionLedger::default(),
        InMemoryTransactionIdRegistry::default(),
    )
}
