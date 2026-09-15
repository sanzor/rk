use rust_decimal::Decimal;

use crate::domain::ids::{ClientId, TransactionId};

/// A debit from a client's account. Rejected if the account does not have
/// sufficient available funds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Withdrawal {
    pub client_id: ClientId,
    pub tx_id: TransactionId,
    pub amount: Decimal,
}
