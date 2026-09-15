use rust_decimal::Decimal;

use crate::domain::ids::{ClientId, TransactionId};

/// A credit to a client's account.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Deposit {
    pub client_id: ClientId,
    pub tx_id: TransactionId,
    pub amount: Decimal,
}
