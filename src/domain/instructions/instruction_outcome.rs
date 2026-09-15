/// What happened when an [`Instruction`](super::instruction::Instruction)
/// was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionOutcome {
    Applied,
    Rejected { reason: RejectionReason },
}

/// Why an instruction was rejected instead of applied.
///
/// A rejection is an expected, well-formed business outcome (e.g. a
/// dispute referencing a transaction that doesn't exist) rather than a
/// system failure, so it is a value here rather than an error type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionReason {
    /// The account is locked (a prior chargeback froze it).
    AccountLocked,
    /// A withdrawal exceeded the account's available funds.
    InsufficientFunds,
    /// A deposit or withdrawal reused a `tx_id` belonging to an accepted
    /// transaction.
    DuplicateTransaction,
    /// A deposit or withdrawal supplied a negative monetary amount.
    InvalidAmount,
    /// A dispute, resolve, or chargeback referenced a `tx_id` with no
    /// matching disputable transaction for that client.
    UnknownTransaction,
    /// A dispute referenced a `tx_id` that is already under dispute.
    TransactionAlreadyDisputed,
    /// A resolve or chargeback referenced a `tx_id` that is not currently
    /// under dispute.
    TransactionNotDisputed,
}
