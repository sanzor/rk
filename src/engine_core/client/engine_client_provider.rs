use crate::domain::{Instruction, InstructionResult};
use crate::engine_core::EngineClientError;

/// What every producer actually depends on to talk to a running engine —
/// never the concrete [`EngineClient`](super::engine_client::EngineClient)
/// directly. Lets a caller (or a test) swap in a double without touching
/// the engine itself.
#[async_trait::async_trait]
pub trait EngineClientProvider: Send + Sync + 'static {
    async fn send_instruction(
        &self,
        instruction: Instruction,
    ) -> Result<InstructionResult, EngineClientError>;
}
