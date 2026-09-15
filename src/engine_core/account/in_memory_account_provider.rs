use std::collections::HashMap;

use super::account_provider::AccountProvider;
use crate::domain::{Account, ClientId};

/// The default [`AccountProvider`]: accounts live in a `HashMap` for the
/// lifetime of the process, with no further persistence.
#[derive(Debug, Default)]
pub struct InMemoryAccountProvider {
    accounts: HashMap<ClientId, Account>,
}

#[async_trait::async_trait]
impl AccountProvider for InMemoryAccountProvider {
    async fn get(&mut self, client_id: ClientId) -> Account {
        self.accounts
            .get(&client_id)
            .copied()
            .unwrap_or_else(|| Account::new(client_id))
    }

    async fn save(&mut self, account: Account) {
        self.accounts.insert(account.client_id, account);
    }
}
