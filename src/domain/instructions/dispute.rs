use crate::domain::ids::{ClientId, TransactionId};

/// A client's claim that `tx_id` was erroneous. Holds the disputed funds
/// without reversing them.
///
/// Assumption: only a deposit can be disputed. A `tx_id` referring to a
/// withdrawal, or to nothing at all, is treated as unknown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dispute {
    pub client_id: ClientId,
    pub tx_id: TransactionId,
}
