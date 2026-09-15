use crate::engine_core::AccountProvider;
use crate::engine_core::TransactionIdRegistry;
use crate::engine_core::transaction_ledger::TransactionLedger;

/// Everything [`EngineHandle::start`](super::EngineHandle::start) needs
/// to start an engine, bundled into one value.
pub struct EngineParams<A: AccountProvider, T: TransactionLedger, R: TransactionIdRegistry> {
    pub accounts: A,
    pub transactions: T,
    pub transaction_ids: R,
}
