use crate::domain::ids::{ClientId, TransactionId};

/// Reverses a disputed transaction and freezes the account.
///
/// Assumption: a locked account rejects every further instruction,
/// including a second dispute lifecycle — it is frozen, not just missing
/// the disputed funds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChargeBack {
    pub client_id: ClientId,
    pub tx_id: TransactionId,
}
