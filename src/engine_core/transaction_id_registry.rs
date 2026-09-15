use crate::domain::TransactionId;

/// Globally reserves IDs for accepted monetary transactions.
///
/// A registry is shared by every engine shard. `reserve` is deliberately
/// synchronous: implementations must complete its check-and-insert atomically
/// and must not hold a lock across any asynchronous account or ledger work.
pub trait TransactionIdRegistry: Send + Sync + 'static {
    /// Reserves `tx_id`, returning `true` only for its first successful use.
    fn reserve(&self, tx_id: TransactionId) -> bool;
}
