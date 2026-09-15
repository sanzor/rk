use rust_decimal::Decimal;

use crate::domain::{ClientId, TransactionId};
use crate::engine_core::transaction_record::DisputableTransaction;

/// Tracks disputable deposits through their dispute lifecycle, so a
/// dispute/resolve/chargeback can look up the amount and dispute status behind
/// a `tx_id`.
///
/// The lifecycle: [`record_deposit`](Self::record_deposit) creates the
/// record (not yet disputed); a dispute
/// [`mark_disputed`](Self::mark_disputed)s it; a resolve or a chargeback
/// then [`remove`](Self::remove)s it entirely — both are terminal, so there's
/// nothing left to track once either happens. Global transaction-ID retention
/// belongs to [`TransactionIdRegistry`](crate::engine_core::TransactionIdRegistry),
/// not this active-dispute ledger.
///
/// The engine is single-consumer (see
/// [`EngineHandle::start`](crate::engine_core::EngineHandle::start)):
/// only one task ever calls these methods, and never concurrently, so
/// implementations don't need interior synchronization.
#[async_trait::async_trait]
pub trait TransactionLedger: Send + 'static {
    async fn record_deposit(&mut self, tx_id: TransactionId, client_id: ClientId, amount: Decimal);

    /// `None` if `tx_id` doesn't name a known, still-tracked deposit.
    async fn lookup_disputable(&mut self, tx_id: TransactionId) -> Option<DisputableTransaction>;

    /// Marks a known deposit as currently disputed. Only ever called
    /// right after a [`lookup_disputable`](Self::lookup_disputable) that
    /// found it not yet disputed.
    async fn mark_disputed(&mut self, tx_id: TransactionId);

    /// Ends a deposit's dispute lifecycle — via resolution or chargeback —
    /// removing it entirely. A no-op if `tx_id` isn't tracked.
    async fn remove(&mut self, tx_id: TransactionId);
}
