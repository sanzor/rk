use std::sync::Arc;

use tokio::sync::mpsc;

use super::engine::Engine;
use super::engine_params::EngineParams;
use crate::engine_core::AccountProvider;
use crate::engine_core::client::ClientFactory;
use crate::engine_core::engine_message::EngineMessage;
use crate::engine_core::transaction_id_registry::TransactionIdRegistry;
use crate::engine_core::transaction_ledger::TransactionLedger;

const MESSAGE_CHANNEL_CAPACITY: usize = 1024;

/// Owns one or more running engine shards.
///
/// `Engine` itself stays entirely private to this crate. [`start`](Self::start)
/// launches one shard; [`start_sharded`](Self::start_sharded) launches a
/// bounded pool. [`client_factory`](Self::client_factory) hands out clients
/// that route every client's instructions to the same shard.
pub struct EngineHandle {
    senders: Arc<[mpsc::Sender<EngineMessage>]>,
}

impl EngineHandle {
    /// Starts a new engine as a background task and returns a handle to
    /// it.
    ///
    /// Nothing here can actually fail — an `Engine` is just plain
    /// in-memory state until instructions start arriving — so there's no
    /// `Result` to thread through, and no `.await` needed at the call
    /// site either.
    pub fn start<A, T, R>(params: EngineParams<A, T, R>) -> Self
    where
        A: AccountProvider,
        T: TransactionLedger,
        R: TransactionIdRegistry,
    {
        Self::from_params([params])
    }

    /// Starts a fixed number of independent engine shards.
    ///
    /// The factory is called once per shard, so each shard receives separate
    /// account and active-dispute storage. It may clone a shared dependency,
    /// such as an in-memory [`TransactionIdRegistry`], when a value must span
    /// all shards. Clients route by `client_id`, preserving per-client order
    /// while allowing different clients to be processed in parallel.
    pub fn start_sharded<A, T, R, F>(shard_count: usize, mut make_params: F) -> Self
    where
        A: AccountProvider,
        T: TransactionLedger,
        R: TransactionIdRegistry,
        F: FnMut() -> EngineParams<A, T, R>,
    {
        assert!(shard_count > 0, "an engine needs at least one shard");

        let params = (0..shard_count).map(|_| make_params());
        Self::from_params(params)
    }

    fn from_params<A, T, R>(params: impl IntoIterator<Item = EngineParams<A, T, R>>) -> Self
    where
        A: AccountProvider,
        T: TransactionLedger,
        R: TransactionIdRegistry,
    {
        let senders: Vec<_> = params
            .into_iter()
            .map(|params| {
                let EngineParams {
                    accounts,
                    transactions,
                    transaction_ids,
                } = params;

                let (sender, receiver) = mpsc::channel(MESSAGE_CHANNEL_CAPACITY);
                let engine = Engine::new(accounts, transactions, transaction_ids);
                tokio::spawn(engine.run(receiver));
                sender
            })
            .collect();

        Self {
            senders: Arc::from(senders),
        }
    }

    /// A factory for minting `EngineClient`s that share this engine's shard
    /// set. Cheap to call as many times as needed.
    pub fn client_factory(&self) -> ClientFactory {
        ClientFactory::new(Arc::clone(&self.senders))
    }
}
