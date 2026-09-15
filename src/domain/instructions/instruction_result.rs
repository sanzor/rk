use super::instruction_outcome::InstructionOutcome;
use crate::domain::account::Account;

/// The outcome of applying a single
/// [`Instruction`](super::instruction::Instruction), paired with the
/// affected account's resulting state.
#[derive(Debug, Clone, PartialEq)]
pub struct InstructionResult {
    pub outcome: InstructionOutcome,
    pub account: Account,
}
