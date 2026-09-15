use crate::domain::{Account, ClientId};

/// Storage for client accounts, abstracted away from the engine's business
/// logic so the backing store can change (in-memory today, a real
/// datastore later) without touching
/// [`Engine`](crate::engine_core::engine::Engine).
///
/// The engine is single-consumer (see
/// [`EngineHandle::start`](crate::engine_core::EngineHandle::start)):
/// only one task ever calls these methods, and never concurrently, so
/// implementations don't need interior synchronization.
#[async_trait::async_trait]
pub trait AccountProvider: Send + 'static {
    /// The account for `client_id`, or a fresh zero-balance one if it
    /// doesn't exist yet. Not persisted until [`save`](Self::save) is
    /// called with it.
    async fn get(&mut self, client_id: ClientId) -> Account;

    async fn save(&mut self, account: Account);
}
