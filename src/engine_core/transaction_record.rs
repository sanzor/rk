use rust_decimal::Decimal;

use crate::domain::ClientId;

/// A previously accepted deposit, as returned by
/// [`TransactionLedger::lookup_disputable`](crate::engine_core::transaction_ledger::TransactionLedger::lookup_disputable).
///
/// Only deposits are disputable (see the assumption documented on
/// [`Dispute`](crate::domain::Dispute)), so this is the shape of any
/// transaction a dispute/resolve/chargeback can reference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisputableTransaction {
    pub client_id: ClientId,
    pub amount: Decimal,
    pub disputed: bool,
}
