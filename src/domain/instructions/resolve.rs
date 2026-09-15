use crate::domain::ids::{ClientId, TransactionId};

/// Releases a prior dispute's held funds back to `available` without
/// reversing the underlying transaction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolve {
    pub client_id: ClientId,
    pub tx_id: TransactionId,
}
