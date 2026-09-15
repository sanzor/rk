use rust_decimal::Decimal;

use crate::domain::ids::ClientId;

/// A single client's asset account.
///
/// Assumption: a client has exactly one asset account, and every
/// instruction for that client moves funds in or out of it.
///
/// This type only holds state and simple arithmetic over it — it does not
/// decide whether an operation is *allowed* (e.g. whether an account is
/// locked, or has sufficient funds). Those business rules live in the
/// engine, which reads `locked`/`available` to decide, then calls the
/// matching mutator here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Account {
    pub client_id: ClientId,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl Account {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            total: Decimal::ZERO,
            locked: false,
        }
    }

    pub fn has_sufficient_available_funds(&self, amount: Decimal) -> bool {
        self.available >= amount
    }

    /// A deposit: credits `available` and `total`.
    pub fn credit(&mut self, amount: Decimal) {
        self.available += amount;
        self.total += amount;
    }

    /// A withdrawal: debits `available` and `total`.
    pub fn debit(&mut self, amount: Decimal) {
        self.available -= amount;
        self.total -= amount;
    }

    /// A dispute opening: moves funds from `available` into `held`.
    /// `total` is unchanged.
    pub fn hold(&mut self, amount: Decimal) {
        self.available -= amount;
        self.held += amount;
    }

    /// A dispute resolving: moves funds from `held` back into `available`.
    /// `total` is unchanged.
    pub fn release(&mut self, amount: Decimal) {
        self.held -= amount;
        self.available += amount;
    }

    /// A chargeback: permanently removes the held funds and freezes the
    /// account.
    pub fn chargeback(&mut self, amount: Decimal) {
        self.held -= amount;
        self.total -= amount;
        self.locked = true;
    }
}
