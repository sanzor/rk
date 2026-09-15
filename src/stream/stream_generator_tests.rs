#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use rust_decimal_macros::dec;

    use super::parse_file;
    use crate::engine_core::{
        EngineClientProvider, EngineHandle, EngineParams, InMemoryAccountProvider,
        InMemoryTransactionIdRegistry, InMemoryTransactionLedger,
    };

    fn spawn_client() -> impl EngineClientProvider {
        EngineHandle::start(EngineParams {
            accounts: InMemoryAccountProvider::default(),
            transactions: InMemoryTransactionLedger::default(),
            transaction_ids: InMemoryTransactionIdRegistry::default(),
        })
        .client()
    }

    fn temp_csv(name: &str, contents: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("cli_test_{}_{name}.csv", std::process::id()));
        fs::write(&path, contents).unwrap();
        path
    }

    #[tokio::test]
    async fn matches_the_brief_example() {
        let path = temp_csv(
            "brief_example",
            "type, client, tx, amount\n\
             deposit, 1, 1, 1.0\n\
             deposit, 2, 2, 2.0\n\
             deposit, 1, 3, 2.0\n\
             withdrawal, 1, 4, 1.5\n\
             withdrawal, 2, 5, 3.0\n",
        );

        let mut accounts = parse_file(&path, &spawn_client()).await.unwrap();
        accounts.sort_by_key(|account| account.client_id);

        assert_eq!(accounts.len(), 2);

        assert_eq!(accounts[0].client_id, 1);
        assert_eq!(accounts[0].available, dec!(1.5));
        assert_eq!(accounts[0].held, dec!(0));
        assert_eq!(accounts[0].total, dec!(1.5));
        assert!(!accounts[0].locked);

        // Client 2's withdrawal of 3.0 exceeds their available 2.0, so it's
        // rejected and their balance is unchanged.
        assert_eq!(accounts[1].client_id, 2);
        assert_eq!(accounts[1].available, dec!(2.0));
        assert_eq!(accounts[1].held, dec!(0));
        assert_eq!(accounts[1].total, dec!(2.0));
        assert!(!accounts[1].locked);

        fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn dispute_resolve_and_chargeback_lifecycle() {
        let path = temp_csv(
            "dispute_lifecycle",
            "type, client, tx, amount\n\
             deposit, 1, 1, 5.0\n\
             dispute, 1, 1,\n\
             resolve, 1, 1,\n\
             deposit, 2, 2, 3.0\n\
             dispute, 2, 2,\n\
             chargeback, 2, 2,\n",
        );

        let mut accounts = parse_file(&path, &spawn_client()).await.unwrap();
        accounts.sort_by_key(|account| account.client_id);

        assert_eq!(accounts.len(), 2);

        // Disputed then resolved: back to normal, nothing held.
        assert_eq!(accounts[0].client_id, 1);
        assert_eq!(accounts[0].available, dec!(5.0));
        assert_eq!(accounts[0].held, dec!(0));
        assert_eq!(accounts[0].total, dec!(5.0));
        assert!(!accounts[0].locked);

        // Disputed then charged back: funds reversed, account frozen.
        assert_eq!(accounts[1].client_id, 2);
        assert_eq!(accounts[1].available, dec!(0));
        assert_eq!(accounts[1].held, dec!(0));
        assert_eq!(accounts[1].total, dec!(0));
        assert!(accounts[1].locked);

        fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn bad_rows_are_skipped_not_fatal() {
        let path = temp_csv(
            "bad_rows",
            "type, client, tx, amount\n\
             deposit, 1, 1, 1.0\n\
             bogus, 2, 2, 1.0\n\
             deposit, 3, 3, notanumber\n",
        );

        let accounts = parse_file(&path, &spawn_client()).await.unwrap();

        // Only the valid row ever reached the engine, so only client 1 shows
        // up — the two bad rows never became instructions at all.
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].client_id, 1);
        assert_eq!(accounts[0].total, dec!(1.0));

        fs::remove_file(path).ok();
    }
}
