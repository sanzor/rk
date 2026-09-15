//! End-to-end tests of the real actor/queue plumbing — submitting
//! through [`EngineClientProvider`], not calling `Engine` directly (see
//! `engine::tests` for that). These prove the queue, `EngineHandle`, and
//! `EngineClient` actually deliver the business logic correctly, on top
//! of what `engine::tests` already proves about the logic itself.
use crate::domain::{
    ChargeBack, Deposit, Dispute, Instruction, InstructionOutcome, RejectionReason, Resolve,
    Withdrawal,
};
use rust_decimal_macros::dec;

use super::EngineClientProvider;
use super::test_support::spawn_engine;

#[tokio::test]
async fn deposit_credits_available_and_total() {
    let client = spawn_engine();

    let result = client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.5),
        }))
        .await
        .unwrap();

    assert_eq!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(1.5));
    assert_eq!(result.account.total, dec!(1.5));
    assert_eq!(result.account.held, dec!(0));
    assert!(!result.account.locked);
}

#[tokio::test]
async fn withdrawal_debits_available_and_total() {
    let client = spawn_engine();

    client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(2.0),
        }))
        .await
        .unwrap();

    let result = client
        .send_instruction(Instruction::Withdrawal(Withdrawal {
            client_id: 1,
            tx_id: 2,
            amount: dec!(0.5),
        }))
        .await
        .unwrap();

    assert_eq!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(1.5));
    assert_eq!(result.account.total, dec!(1.5));
}

#[tokio::test]
async fn withdrawal_with_insufficient_funds_is_rejected() {
    let client = spawn_engine();

    let result = client
        .send_instruction(Instruction::Withdrawal(Withdrawal {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.0),
        }))
        .await
        .unwrap();

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::InsufficientFunds
        }
    );
    assert_eq!(result.account.total, dec!(0));
}

#[tokio::test]
async fn dispute_holds_funds_without_changing_total() {
    let client = spawn_engine();

    client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.0),
        }))
        .await
        .unwrap();

    let result = client
        .send_instruction(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await
        .unwrap();

    assert_eq!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(0));
    assert_eq!(result.account.held, dec!(1.0));
    assert_eq!(result.account.total, dec!(1.0));
}

#[tokio::test]
async fn dispute_on_unknown_transaction_is_rejected() {
    let client = spawn_engine();

    let result = client
        .send_instruction(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 999,
        }))
        .await
        .unwrap();

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::UnknownTransaction
        }
    );
}

#[tokio::test]
async fn resolve_releases_held_funds() {
    let client = spawn_engine();

    client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.0),
        }))
        .await
        .unwrap();
    client
        .send_instruction(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await
        .unwrap();

    let result = client
        .send_instruction(Instruction::Resolve(Resolve {
            client_id: 1,
            tx_id: 1,
        }))
        .await
        .unwrap();

    assert_eq!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(1.0));
    assert_eq!(result.account.held, dec!(0));
    assert!(!result.account.locked);
}

#[tokio::test]
async fn resolve_on_non_disputed_transaction_is_rejected() {
    let client = spawn_engine();

    client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.0),
        }))
        .await
        .unwrap();

    let result = client
        .send_instruction(Instruction::Resolve(Resolve {
            client_id: 1,
            tx_id: 1,
        }))
        .await
        .unwrap();

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::TransactionNotDisputed
        }
    );
}

#[tokio::test]
async fn chargeback_locks_account_and_reverses_held_funds() {
    let client = spawn_engine();

    client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.0),
        }))
        .await
        .unwrap();
    client
        .send_instruction(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await
        .unwrap();

    let result = client
        .send_instruction(Instruction::ChargeBack(ChargeBack {
            client_id: 1,
            tx_id: 1,
        }))
        .await
        .unwrap();

    assert_eq!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(0));
    assert_eq!(result.account.held, dec!(0));
    assert_eq!(result.account.total, dec!(0));
    assert!(result.account.locked);
}

#[tokio::test]
async fn locked_account_rejects_further_deposits() {
    let client = spawn_engine();

    client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.0),
        }))
        .await
        .unwrap();
    client
        .send_instruction(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await
        .unwrap();
    client
        .send_instruction(Instruction::ChargeBack(ChargeBack {
            client_id: 1,
            tx_id: 1,
        }))
        .await
        .unwrap();

    let result = client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 2,
            amount: dec!(5.0),
        }))
        .await
        .unwrap();

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::AccountLocked
        }
    );
    assert_eq!(result.account.total, dec!(0));
}

#[tokio::test]
async fn duplicate_transaction_id_is_rejected() {
    let client = spawn_engine();

    client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.0),
        }))
        .await
        .unwrap();

    let result = client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(9.0),
        }))
        .await
        .unwrap();

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::DuplicateTransaction
        }
    );
    assert_eq!(result.account.total, dec!(1.0));
}
