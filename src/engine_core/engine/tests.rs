//! Edge cases mined from `Rust Coding Challenge.pdf`, testing [`Engine`]
//! itself directly, not through
//! [`EngineClient`](crate::engine_core::client::EngineClient)'s queue: these are unit
//! tests of the business logic, not of the actor plumbing (that's what
//! `client::tests` is for). Property-based invariants live in
//! `property_tests`, not here.
//!
//! Each test's doc comment says whether it's checking something the
//! document states explicitly, or a documented assumption we've made
//! where the document is silent — see the relevant type's own doc
//! comment (`crate::domain::Dispute`, `crate::engine_core::transaction_ledger::TransactionLedger`)
//! for the assumption itself.
use crate::domain::{
    ChargeBack, Deposit, Dispute, Instruction, InstructionOutcome, RejectionReason, Resolve,
    Withdrawal,
};
use rust_decimal_macros::dec;

use super::test_support::new_engine;

/// Spec: "If a client does not have sufficient available funds the
/// withdrawal should fail." Withdrawing *exactly* what's available is not
/// insufficient, so this boundary case must succeed, not be rejected.
#[tokio::test]
async fn withdrawal_of_exactly_available_amount_succeeds() {
    let mut engine = new_engine();

    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(5.0),
        }))
        .await;

    let result = engine
        .apply(Instruction::Withdrawal(Withdrawal {
            client_id: 1,
            tx_id: 2,
            amount: dec!(5.0),
        }))
        .await;

    assert_eq!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(0));
    assert_eq!(result.account.total, dec!(0));
}

/// Spec: dispute's rule is "available funds should decrease by the
/// amount disputed" — unlike withdrawal, there's no accompanying
/// sufficient-funds requirement anywhere in that section. So disputing a
/// deposit whose funds have already been (fully) withdrawn is expected to
/// push `available` negative, mirroring a real chargeback landing after
/// the money is already spent.
#[tokio::test]
async fn dispute_can_drive_available_negative() {
    let mut engine = new_engine();

    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(5.0),
        }))
        .await;
    engine
        .apply(Instruction::Withdrawal(Withdrawal {
            client_id: 1,
            tx_id: 2,
            amount: dec!(5.0),
        }))
        .await;

    let result = engine
        .apply(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await;

    assert_eq!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(-5.0));
    assert_eq!(result.account.held, dec!(5.0));
    assert_eq!(result.account.total, dec!(0));
}

/// Spec: resolve/chargeback should each be ignored if "the tx specified
/// doesn't exist" — a condition stated separately from "the tx isn't
/// under dispute" (covered by other tests). This is the "doesn't exist at
/// all" half, for chargeback specifically.
#[tokio::test]
async fn chargeback_on_unknown_transaction_is_ignored() {
    let mut engine = new_engine();

    let result = engine
        .apply(Instruction::ChargeBack(ChargeBack {
            client_id: 1,
            tx_id: 999,
        }))
        .await;

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::UnknownTransaction
        }
    );
}

/// Spec: same "doesn't exist" condition as above, for resolve.
#[tokio::test]
async fn resolve_on_unknown_transaction_is_ignored() {
    let mut engine = new_engine();

    let result = engine
        .apply(Instruction::Resolve(Resolve {
            client_id: 1,
            tx_id: 999,
        }))
        .await;

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::UnknownTransaction
        }
    );
}

/// Spec: "the tx isn't under dispute" is a *separate* ignore condition
/// from "doesn't exist" for chargeback too — a real, known deposit that
/// was never disputed can't be charged back directly.
#[tokio::test]
async fn chargeback_on_undisputed_transaction_is_ignored() {
    let mut engine = new_engine();

    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.0),
        }))
        .await;

    let result = engine
        .apply(Instruction::ChargeBack(ChargeBack {
            client_id: 1,
            tx_id: 1,
        }))
        .await;

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::TransactionNotDisputed
        }
    );
}

/// Spec: chargeback's rule is "held funds and total funds should
/// decrease" — `available` is never mentioned, and shouldn't move. Uses a
/// second, undisputed deposit so `available` is provably nonzero and
/// unaffected by the chargeback of the first one.
#[tokio::test]
async fn chargeback_does_not_touch_available() {
    let mut engine = new_engine();

    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(5.0),
        }))
        .await;
    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 2,
            amount: dec!(3.0),
        }))
        .await;
    engine
        .apply(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await;

    let result = engine
        .apply(Instruction::ChargeBack(ChargeBack {
            client_id: 1,
            tx_id: 1,
        }))
        .await;

    assert_eq!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(3.0));
    assert_eq!(result.account.held, dec!(0));
    assert_eq!(result.account.total, dec!(3.0));
    assert!(result.account.locked);
}

/// Transaction ids are globally unique, so a dispute naming the wrong
/// `client_id` for a real `tx_id` must not succeed by accident — it has
/// to be indistinguishable from disputing an unknown transaction,
/// otherwise one client could hold another client's funds.
#[tokio::test]
async fn dispute_with_mismatched_client_id_is_ignored() {
    let mut engine = new_engine();

    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.0),
        }))
        .await;

    let result = engine
        .apply(Instruction::Dispute(Dispute {
            client_id: 2,
            tx_id: 1,
        }))
        .await;

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::UnknownTransaction
        }
    );

    // And client 1's deposit is untouched by the misdirected dispute.
    let check = engine
        .apply(Instruction::Withdrawal(Withdrawal {
            client_id: 1,
            tx_id: 2,
            amount: dec!(1.0),
        }))
        .await;
    assert_eq!(check.outcome, InstructionOutcome::Applied);
}

/// Assumption (documented on `domain::Dispute`): only a deposit is
/// disputable. A withdrawal's `tx_id` was never recorded in the ledger,
/// so disputing it is indistinguishable from disputing a `tx_id` that
/// never existed at all.
#[tokio::test]
async fn disputing_a_withdrawal_is_ignored() {
    let mut engine = new_engine();

    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(5.0),
        }))
        .await;
    engine
        .apply(Instruction::Withdrawal(Withdrawal {
            client_id: 1,
            tx_id: 2,
            amount: dec!(2.0),
        }))
        .await;

    let result = engine
        .apply(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 2,
        }))
        .await;

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::UnknownTransaction
        }
    );
}

/// `RejectionReason::TransactionAlreadyDisputed` exists specifically to
/// stop a second dispute on a transaction that's already held — otherwise
/// the same funds could be double-held or double-reversed.
#[tokio::test]
async fn disputing_an_already_disputed_transaction_is_rejected() {
    let mut engine = new_engine();

    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(5.0),
        }))
        .await;
    engine
        .apply(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await;

    let result = engine
        .apply(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await;

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::TransactionAlreadyDisputed
        }
    );
    // Held funds are still exactly the original amount, not doubled.
    assert_eq!(result.account.held, dec!(5.0));
}

/// Assumption (documented on `TransactionLedger`): resolve closes a
/// transaction's dispute lifecycle for good, so it can't be disputed a
/// second time after being resolved once.
#[tokio::test]
async fn disputing_a_resolved_transaction_again_is_ignored() {
    let mut engine = new_engine();

    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(5.0),
        }))
        .await;
    engine
        .apply(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await;
    engine
        .apply(Instruction::Resolve(Resolve {
            client_id: 1,
            tx_id: 1,
        }))
        .await;

    let result = engine
        .apply(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await;

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::UnknownTransaction
        }
    );
}

/// Spec never says a zero amount is invalid — only that amounts have up
/// to 4 decimal places. A zero-amount deposit should apply as a no-op,
/// not be treated as an error.
#[tokio::test]
async fn zero_amount_deposit_is_applied() {
    let mut engine = new_engine();

    let result = engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(0),
        }))
        .await;

    assert_eq!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(0));
    assert_eq!(result.account.total, dec!(0));
}

/// A monetary amount must not invert an operation.  In particular, a
/// negative withdrawal must not credit an otherwise empty account.
///
/// This is a robustness requirement rather than a lifecycle rule: CSV
/// deserialization accepts signed `Decimal` values, so the engine must
/// defend this invariant itself.
#[tokio::test]
async fn negative_withdrawal_does_not_credit_an_account() {
    let mut engine = new_engine();

    let result = engine
        .apply(Instruction::Withdrawal(Withdrawal {
            client_id: 1,
            tx_id: 1,
            amount: dec!(-5.0),
        }))
        .await;

    assert_ne!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(0));
    assert_eq!(result.account.total, dec!(0));
}

/// The counterpart to `negative_withdrawal_does_not_credit_an_account`:
/// a negative deposit must not debit the account or create a negative
/// balance.
#[tokio::test]
async fn negative_deposit_does_not_debit_an_account() {
    let mut engine = new_engine();

    let result = engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(-5.0),
        }))
        .await;

    assert_ne!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(0));
    assert_eq!(result.account.total, dec!(0));
}

/// Transaction IDs are global identifiers, not merely keys for deposits
/// that have not yet completed their dispute lifecycle.  A withdrawal must
/// reserve its ID too, so a later deposit cannot reuse it.
#[tokio::test]
async fn a_deposit_cannot_reuse_a_withdrawals_transaction_id() {
    let mut engine = new_engine();

    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(10.0),
        }))
        .await;
    engine
        .apply(Instruction::Withdrawal(Withdrawal {
            client_id: 1,
            tx_id: 2,
            amount: dec!(5.0),
        }))
        .await;

    let result = engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 2,
            amount: dec!(7.0),
        }))
        .await;

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::DuplicateTransaction,
        }
    );
    assert_eq!(result.account.total, dec!(5.0));
}

/// A transaction keeps its identity after a resolve.  Removing its
/// disputable state must not make its global ID available for reuse.
#[tokio::test]
async fn a_deposit_cannot_reuse_a_resolved_transaction_id() {
    let mut engine = new_engine();

    for instruction in [
        Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(5.0),
        }),
        Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }),
        Instruction::Resolve(Resolve {
            client_id: 1,
            tx_id: 1,
        }),
    ] {
        assert_eq!(
            engine.apply(instruction).await.outcome,
            InstructionOutcome::Applied
        );
    }

    let result = engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(7.0),
        }))
        .await;

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::DuplicateTransaction,
        }
    );
    assert_eq!(result.account.total, dec!(5.0));
}

/// Spec: "precision of up to four places past the decimal... should
/// output values with the same level of precision." A full
/// deposit-dispute-resolve round trip must not lose or drift precision.
#[tokio::test]
async fn precision_survives_a_full_dispute_lifecycle() {
    let mut engine = new_engine();

    engine
        .apply(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(1.2345),
        }))
        .await;
    engine
        .apply(Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }))
        .await;

    let result = engine
        .apply(Instruction::Resolve(Resolve {
            client_id: 1,
            tx_id: 1,
        }))
        .await;

    assert_eq!(result.outcome, InstructionOutcome::Applied);
    assert_eq!(result.account.available, dec!(1.2345));
    assert_eq!(result.account.held, dec!(0));
    assert_eq!(result.account.total, dec!(1.2345));
}

/// Spec: "If a client doesn't exist create a new record" — this applies
/// even when the very first thing referencing them ends up rejected. The
/// account still shows up (at zero) rather than being entirely absent
/// from the output.
#[tokio::test]
async fn a_rejected_first_instruction_still_yields_a_zero_balance_record() {
    let mut engine = new_engine();

    let result = engine
        .apply(Instruction::Withdrawal(Withdrawal {
            client_id: 42,
            tx_id: 1,
            amount: dec!(1.0),
        }))
        .await;

    assert_eq!(
        result.outcome,
        InstructionOutcome::Rejected {
            reason: RejectionReason::InsufficientFunds
        }
    );
    assert_eq!(result.account.client_id, 42);
    assert_eq!(result.account.total, dec!(0));
    assert!(!result.account.locked);
}

/// A sequence of several `apply` calls in a row must handle a full
/// dispute-then-chargeback lifecycle the same way as any other sequence.
#[tokio::test]
async fn a_full_dispute_lifecycle_applied_in_sequence() {
    let mut engine = new_engine();

    let mut results = Vec::new();
    for instruction in [
        Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(5.0),
        }),
        Instruction::Dispute(Dispute {
            client_id: 1,
            tx_id: 1,
        }),
        Instruction::ChargeBack(ChargeBack {
            client_id: 1,
            tx_id: 1,
        }),
    ] {
        results.push(engine.apply(instruction).await);
    }

    assert_eq!(results.len(), 3);
    assert!(
        results
            .iter()
            .all(|result| result.outcome == InstructionOutcome::Applied)
    );

    let final_account = &results[2].account;
    assert_eq!(final_account.available, dec!(0));
    assert_eq!(final_account.held, dec!(0));
    assert_eq!(final_account.total, dec!(0));
    assert!(final_account.locked);
}
