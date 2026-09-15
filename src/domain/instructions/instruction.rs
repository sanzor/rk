use super::chargeback::ChargeBack;
use super::deposit::Deposit;
use super::dispute::Dispute;
use super::resolve::Resolve;
use super::withdrawal::Withdrawal;

/// A single instruction from the transaction stream.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instruction {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    ChargeBack(ChargeBack),
}
