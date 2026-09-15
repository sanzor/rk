use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use super::engine_client_error::EngineClientError;
use super::engine_client_provider::EngineClientProvider;
use crate::domain::{Instruction, InstructionResult};
use crate::engine_core::engine_message::EngineMessage;

/// A cheap, cloneable handle to a running
/// [`Engine`](crate::engine_core::engine::Engine).
///
/// This is how every producer submits work: send an instruction, get back
/// the same [`InstructionResult`] a direct call would have returned. The
/// queue underneath is an implementation detail; from the caller's side
/// this still reads as an ordinary async request/response.
#[derive(Debug, Clone)]
pub struct EngineClient {
    senders: Arc<[mpsc::Sender<EngineMessage>]>,
}

impl EngineClient {
    pub(crate) fn new(senders: Arc<[mpsc::Sender<EngineMessage>]>) -> Self {
        Self { senders }
    }

    fn sender_for(&self, instruction: &Instruction) -> &mpsc::Sender<EngineMessage> {
        let client_id = match instruction {
            Instruction::Deposit(instruction) => instruction.client_id,
            Instruction::Withdrawal(instruction) => instruction.client_id,
            Instruction::Dispute(instruction) => instruction.client_id,
            Instruction::Resolve(instruction) => instruction.client_id,
            Instruction::ChargeBack(instruction) => instruction.client_id,
        };
        let shard = usize::from(client_id) % self.senders.len();

        &self.senders[shard]
    }
}

#[async_trait::async_trait]
impl EngineClientProvider for EngineClient {
    async fn send_instruction(
        &self,
        instruction: Instruction,
    ) -> Result<InstructionResult, EngineClientError> {
        let (respond_to, response) = oneshot::channel();

        self.sender_for(&instruction)
            .send(EngineMessage::Instruction {
                instruction,
                respond_to,
            })
            .await
            .map_err(|error| EngineClientError::SendFailed(error.to_string()))?;

        response.await.map_err(EngineClientError::ReplyLost)
    }
}
