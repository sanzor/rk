//! Property-based invariants for [`Engine`], checked against randomly
//! generated instruction sequences rather than hand-picked examples. Kept
//! separate from `tests` (the example-based edge cases) since these are
//! a different kind of test: each one runs ~256 generated cases, not one
//! fixed scenario.
use std::collections::HashSet;

use crate::domain::{
    ChargeBack, ClientId, Deposit, Dispute, Instruction, Resolve, TransactionId, Withdrawal,
};
use proptest::prelude::*;
use rust_decimal::Decimal;

use super::test_support::new_engine;

/// A small, bounded space of client/transaction ids so random sequences
/// actually collide — a dispute/resolve/chargeback that always names a
/// fresh, never-seen id would only ever exercise the "unknown
/// transaction" path.
fn client_id() -> impl Strategy<Value = ClientId> {
    1u16..=3
}

fn tx_id() -> impl Strategy<Value = TransactionId> {
    1u32..=20
}

/// Amounts with up to 4 decimal places, matching the brief's stated
/// precision — built from an integer count of ten-thousandths so the
/// resulting `Decimal` is always exact, never approximated.
fn amount() -> impl Strategy<Value = Decimal> {
    (0u32..=1_000_000u32).prop_map(|ten_thousandths| Decimal::new(i64::from(ten_thousandths), 4))
}

fn instruction() -> impl Strategy<Value = Instruction> {
    prop_oneof![
        (client_id(), tx_id(), amount()).prop_map(|(client_id, tx_id, amount)| {
            Instruction::Deposit(Deposit {
                client_id,
                tx_id,
                amount,
            })
        }),
        (client_id(), tx_id(), amount()).prop_map(|(client_id, tx_id, amount)| {
            Instruction::Withdrawal(Withdrawal {
                client_id,
                tx_id,
                amount,
            })
        }),
        (client_id(), tx_id())
            .prop_map(|(client_id, tx_id)| Instruction::Dispute(Dispute { client_id, tx_id })),
        (client_id(), tx_id())
            .prop_map(|(client_id, tx_id)| Instruction::Resolve(Resolve { client_id, tx_id })),
        (client_id(), tx_id()).prop_map(|(client_id, tx_id)| Instruction::ChargeBack(ChargeBack {
            client_id,
            tx_id
        })),
    ]
}

proptest! {
    /// The spec states the account model as three equivalent equations
    /// (available = total - held, held = total - available, total =
    /// available + held). This must hold after *every single*
    /// instruction — applied or rejected, for any sequence at all — it's
    /// the one invariant the whole model is built on.
    #[test]
    fn total_always_equals_available_plus_held(
        instructions in prop::collection::vec(instruction(), 1..30)
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let mut engine = new_engine();
            for instruction in instructions {
                let result = engine.apply(instruction).await;
                prop_assert_eq!(
                    result.account.total,
                    result.account.available + result.account.held
                );
            }
            Ok(())
        })?;
    }

    /// "If a chargeback occurs the client's account should be
    /// immediately frozen" — and nothing anywhere unlocks an account.
    /// Once locked, it must stay locked for the rest of any sequence.
    #[test]
    fn locked_is_permanent(
        instructions in prop::collection::vec(instruction(), 1..30)
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let mut engine = new_engine();
            let mut ever_locked: HashSet<ClientId> = HashSet::new();

            for instruction in instructions {
                let client_id = match instruction {
                    Instruction::Deposit(Deposit { client_id, .. })
                    | Instruction::Withdrawal(Withdrawal { client_id, .. })
                    | Instruction::Dispute(Dispute { client_id, .. })
                    | Instruction::Resolve(Resolve { client_id, .. })
                    | Instruction::ChargeBack(ChargeBack { client_id, .. }) => client_id,
                };
                let result = engine.apply(instruction).await;

                if ever_locked.contains(&client_id) {
                    prop_assert!(result.account.locked);
                }
                if result.account.locked {
                    ever_locked.insert(client_id);
                }
            }
            Ok(())
        })?;
    }
}
