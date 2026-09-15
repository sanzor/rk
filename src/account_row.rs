use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::{Account, ClientId};

/// The output CSV's row shape — `client,available,held,total,locked` —
/// kept separate from [`Account`] because the column is literally
/// `client`, not `client_id`: [`Account`] keeps `client_id` to stay
/// consistent with the rest of the domain's naming; this type exists only
/// to match the brief's output format.
#[derive(Debug, Serialize)]
pub struct AccountRow {
    pub client: ClientId,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl From<Account> for AccountRow {
    fn from(account: Account) -> Self {
        Self {
            client: account.client_id,
            available: account.available,
            held: account.held,
            total: account.total,
            locked: account.locked,
        }
    }
}
