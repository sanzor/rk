use tokio::sync::oneshot;

use crate::domain::{Instruction, InstructionResult};

/// One unit of work sent to a running [`Engine`](super::engine::Engine)
/// over its queue, paired with a one-shot reply channel so the sender can
/// still get its result back despite going through a queue instead of a
/// direct call.
///
///
///
///
pub(crate) enum EngineMessage {
    Instruction {
        instruction: Instruction,
        respond_to: oneshot::Sender<InstructionResult>,
    },
}
