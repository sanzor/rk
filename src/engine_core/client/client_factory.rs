use std::sync::Arc;

use tokio::sync::mpsc;

use super::engine_client::EngineClient;
use crate::engine_core::engine_message::EngineMessage;

/// Mints [`EngineClient`]s on demand, every one sharing the same bounded set
/// of underlying queues to running engine shards. Obtained from
/// [`EngineHandle::client_factory`](crate::engine_core::EngineHandle::client_factory) —
/// cheap to clone and hand out to as many producers as needed.
#[derive(Debug, Clone)]
pub struct ClientFactory {
    senders: Arc<[mpsc::Sender<EngineMessage>]>,
}

impl ClientFactory {
    pub(crate) fn new(senders: Arc<[mpsc::Sender<EngineMessage>]>) -> Self {
        Self { senders }
    }

    /// A new, independent client a producer uses to submit work. Cheap to
    /// call as many times as needed — every client shares the same
    /// underlying shard set. Nothing here can actually fail: minting a client
    /// is just cloning channel handles, not touching the engines themselves.
    pub fn get_client(&self) -> EngineClient {
        EngineClient::new(Arc::clone(&self.senders))
    }
}
