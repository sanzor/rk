use rust_decimal::Decimal;
use tokio::sync::mpsc;

use crate::domain::{
    ChargeBack, Deposit, Dispute, Instruction, InstructionOutcome, InstructionResult,
    RejectionReason, Resolve, Withdrawal,
};
use crate::engine_core::AccountProvider;
use crate::engine_core::engine_message::EngineMessage;
use crate::engine_core::transaction_id_registry::TransactionIdRegistry;
use crate::engine_core::transaction_ledger::TransactionLedger;

/// Owns all engine state and applies instructions one at a time.
///
/// Spawned as a single background task by
/// [`EngineHandle::start`](super::engine_handle::EngineHandle::start): because
/// only one task ever holds an `Engine`, and it only ever processes one
/// message before moving to the next, `accounts` and `transactions` never
/// see concurrent access — no locking required.
///
/// Stays private to this crate entirely — [`EngineHandle`](super::engine_handle::EngineHandle)
/// is the only thing anyone outside ever touches.
///
/// Each instruction kind gets its own method, fetching and checking only
/// what it actually needs, then mutating the account directly — no
/// generic "state" bundle, no separate math-only layer: the checks and
/// the arithmetic that depend on them belong together.
pub(super) struct Engine<A: AccountProvider, T: TransactionLedger, R: TransactionIdRegistry> {
    accounts: A,
    transactions: T,
    transaction_ids: R,
}

impl<A: AccountProvider, T: TransactionLedger, R: TransactionIdRegistry> Engine<A, T, R> {
    pub(super) fn new(accounts: A, transactions: T, transaction_ids: R) -> Self {
        Self {
            accounts,
            transactions,
            transaction_ids,
        }
    }

    pub(super) async fn apply(&mut self, instruction: Instruction) -> InstructionResult {
        match instruction {
            Instruction::Deposit(deposit) => self.apply_deposit(deposit).await,
            Instruction::Withdrawal(withdrawal) => self.apply_withdrawal(withdrawal).await,
            Instruction::Dispute(dispute) => self.apply_dispute(dispute).await,
            Instruction::Resolve(resolve) => self.apply_resolve(resolve).await,
            Instruction::ChargeBack(chargeback) => self.apply_chargeback(chargeback).await,
        }
    }

    async fn apply_deposit(&mut self, deposit: Deposit) -> InstructionResult {
        let mut account = self.accounts.get(deposit.client_id).await;

        if deposit.amount < Decimal::ZERO {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::InvalidAmount,
                },
                account,
            };
        }

        if account.locked {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::AccountLocked,
                },
                account,
            };
        }

        if !self.transaction_ids.reserve(deposit.tx_id) {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::DuplicateTransaction,
                },
                account,
            };
        }

        account.credit(deposit.amount);
        self.accounts.save(account).await;
        self.transactions
            .record_deposit(deposit.tx_id, deposit.client_id, deposit.amount)
            .await;

        InstructionResult {
            outcome: InstructionOutcome::Applied,
            account,
        }
    }

    async fn apply_withdrawal(&mut self, withdrawal: Withdrawal) -> InstructionResult {
        let mut account = self.accounts.get(withdrawal.client_id).await;

        if withdrawal.amount < Decimal::ZERO {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::InvalidAmount,
                },
                account,
            };
        }

        if account.locked {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::AccountLocked,
                },
                account,
            };
        }

        if !account.has_sufficient_available_funds(withdrawal.amount) {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::InsufficientFunds,
                },
                account,
            };
        }

        if !self.transaction_ids.reserve(withdrawal.tx_id) {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::DuplicateTransaction,
                },
                account,
            };
        }

        account.debit(withdrawal.amount);
        self.accounts.save(account).await;

        InstructionResult {
            outcome: InstructionOutcome::Applied,
            account,
        }
    }

    async fn apply_dispute(&mut self, dispute: Dispute) -> InstructionResult {
        let mut account = self.accounts.get(dispute.client_id).await;

        if account.locked {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::AccountLocked,
                },
                account,
            };
        }

        let Some(disputable) = self.transactions.lookup_disputable(dispute.tx_id).await else {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::UnknownTransaction,
                },
                account,
            };
        };

        if disputable.client_id != dispute.client_id {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::UnknownTransaction,
                },
                account,
            };
        }

        if disputable.disputed {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::TransactionAlreadyDisputed,
                },
                account,
            };
        }

        account.hold(disputable.amount);
        self.accounts.save(account).await;
        self.transactions.mark_disputed(dispute.tx_id).await;

        InstructionResult {
            outcome: InstructionOutcome::Applied,
            account,
        }
    }

    async fn apply_resolve(&mut self, resolve: Resolve) -> InstructionResult {
        let mut account = self.accounts.get(resolve.client_id).await;

        if account.locked {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::AccountLocked,
                },
                account,
            };
        }

        let Some(disputable) = self.transactions.lookup_disputable(resolve.tx_id).await else {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::UnknownTransaction,
                },
                account,
            };
        };

        if disputable.client_id != resolve.client_id {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::UnknownTransaction,
                },
                account,
            };
        }

        if !disputable.disputed {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::TransactionNotDisputed,
                },
                account,
            };
        }

        account.release(disputable.amount);
        self.accounts.save(account).await;
        self.transactions.remove(resolve.tx_id).await;

        InstructionResult {
            outcome: InstructionOutcome::Applied,
            account,
        }
    }

    async fn apply_chargeback(&mut self, chargeback: ChargeBack) -> InstructionResult {
        let mut account = self.accounts.get(chargeback.client_id).await;

        if account.locked {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::AccountLocked,
                },
                account,
            };
        }

        let Some(disputable) = self.transactions.lookup_disputable(chargeback.tx_id).await else {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::UnknownTransaction,
                },
                account,
            };
        };

        if disputable.client_id != chargeback.client_id {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::UnknownTransaction,
                },
                account,
            };
        }

        if !disputable.disputed {
            return InstructionResult {
                outcome: InstructionOutcome::Rejected {
                    reason: RejectionReason::TransactionNotDisputed,
                },
                account,
            };
        }

        account.chargeback(disputable.amount);
        self.accounts.save(account).await;
        self.transactions.remove(chargeback.tx_id).await;

        InstructionResult {
            outcome: InstructionOutcome::Applied,
            account,
        }
    }

    pub(super) async fn run(mut self, mut receiver: mpsc::Receiver<EngineMessage>) {
        while let Some(EngineMessage::Instruction {
            instruction,
            respond_to,
        }) = receiver.recv().await
        {
            let result: InstructionResult = self.apply(instruction).await;
            let _ = respond_to.send(result);
        }
        // `receiver.recv()` returns `None` once every `EngineClient` has
        // been dropped — a graceful way for this task to end.
    }
}
