//! Scenarios with several concurrent producers submitting to the *same*
//! shared engine at once, via cloned handles to one
//! [`EngineClient`](super::EngineClient) — proving the single-consumer
//! actor design (see `engine::Engine`) actually delivers what it
//! promises under real concurrency, not just in the sequential tests in
//! `tests`. Every assertion here is deliberately order-independent: a
//! race's *outcome* is non-deterministic by nature, but these properties
//! (nothing is lost, nothing is double-applied, clients don't leak into
//! each other) must hold no matter how the race resolves.
use std::sync::Arc;

use crate::domain::{Deposit, Instruction, InstructionOutcome, RejectionReason, Withdrawal};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::EngineClientProvider;
use super::test_support::{spawn_engine, spawn_sharded_engine};

/// Many producers depositing to the *same* client concurrently must all
/// land — no lost updates, regardless of how the tasks interleave at the
/// queue.
#[tokio::test]
async fn concurrent_deposits_from_many_producers_all_land() {
    let client = Arc::new(spawn_engine());
    const PRODUCERS: u32 = 50;

    let mut handles = Vec::with_capacity(PRODUCERS as usize);
    for tx_id in 1..=PRODUCERS {
        let client = Arc::clone(&client);
        handles.push(tokio::spawn(async move {
            client
                .send_instruction(Instruction::Deposit(Deposit {
                    client_id: 1,
                    tx_id,
                    amount: dec!(1.0),
                }))
                .await
        }));
    }

    for handle in handles {
        let result = handle.await.unwrap().unwrap();
        assert_eq!(result.outcome, InstructionOutcome::Applied);
    }

    // A last, zero-amount deposit just to read back the final total.
    let final_result = client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: PRODUCERS + 1,
            amount: dec!(0),
        }))
        .await
        .unwrap();

    assert_eq!(final_result.account.total, Decimal::from(PRODUCERS));
}

/// Several producers racing to submit a deposit under the *same* `tx_id`
/// (e.g. a naive retry after a timeout) must resolve to exactly one
/// winner — the single-consumer queue means there's no window where two
/// of them could both pass the duplicate check before either is
/// recorded, unlike a lock-based check-then-act design would risk.
#[tokio::test]
async fn concurrent_duplicate_deposits_only_one_succeeds() {
    let client = Arc::new(spawn_engine());
    const RACERS: usize = 20;

    let mut handles = Vec::with_capacity(RACERS);
    for _ in 0..RACERS {
        let client = Arc::clone(&client);
        handles.push(tokio::spawn(async move {
            client
                .send_instruction(Instruction::Deposit(Deposit {
                    client_id: 1,
                    tx_id: 1,
                    amount: dec!(10.0),
                }))
                .await
        }));
    }

    let mut applied = 0;
    let mut rejected_as_duplicate = 0;
    let mut unexpected = Vec::new();

    for handle in handles {
        let result = handle.await.unwrap().unwrap();
        match result.outcome {
            InstructionOutcome::Applied => applied += 1,
            InstructionOutcome::Rejected {
                reason: RejectionReason::DuplicateTransaction,
            } => rejected_as_duplicate += 1,
            other => unexpected.push(other),
        }
    }

    assert!(unexpected.is_empty(), "unexpected outcomes: {unexpected:?}");
    assert_eq!(applied, 1);
    assert_eq!(rejected_as_duplicate, RACERS - 1);

    let final_result = client
        .send_instruction(Instruction::Withdrawal(Withdrawal {
            client_id: 1,
            tx_id: 2,
            amount: dec!(10.0),
        }))
        .await
        .unwrap();
    // Only one deposit's worth of funds ever landed, not RACERS-many.
    assert_eq!(final_result.outcome, InstructionOutcome::Applied);
    assert_eq!(final_result.account.total, dec!(0));
}

/// Transaction IDs must stay unique even when requests route to different
/// client shards. Client IDs 1 and 2 select different shards in this
/// two-shard pool, so this exercises the shared transaction-ID registry,
/// rather than a duplicate check confined to one actor.
#[tokio::test]
async fn duplicate_transaction_ids_are_rejected_across_shards() {
    let client = Arc::new(spawn_sharded_engine(2));

    let first = {
        let client = Arc::clone(&client);
        tokio::spawn(async move {
            client
                .send_instruction(Instruction::Deposit(Deposit {
                    client_id: 1,
                    tx_id: 42,
                    amount: dec!(10.0),
                }))
                .await
        })
    };
    let second = {
        let client = Arc::clone(&client);
        tokio::spawn(async move {
            client
                .send_instruction(Instruction::Deposit(Deposit {
                    client_id: 2,
                    tx_id: 42,
                    amount: dec!(10.0),
                }))
                .await
        })
    };

    let results = [
        first.await.unwrap().unwrap(),
        second.await.unwrap().unwrap(),
    ];
    assert_eq!(
        results
            .iter()
            .filter(|result| result.outcome == InstructionOutcome::Applied)
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| {
                result.outcome
                    == InstructionOutcome::Rejected {
                        reason: RejectionReason::DuplicateTransaction,
                    }
            })
            .count(),
        1
    );
}

/// Many producers racing to withdraw more, in total, than the account
/// holds must never overdraw it: exactly enough withdrawals succeed to
/// exhaust the available funds, and the rest are rejected — regardless of
/// which ones happen to win the race.
#[tokio::test]
async fn concurrent_withdrawals_never_overdraw() {
    // The same client must always land on one shard, even when the engine is
    // a pool. This reuses the overdraw race to prove that per-client
    // serialization survives routing.
    let client = Arc::new(spawn_sharded_engine(4));

    client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 1,
            amount: dec!(10.0),
        }))
        .await
        .unwrap();

    const WITHDRAWAL_ATTEMPTS: u32 = 20;
    let mut handles = Vec::with_capacity(WITHDRAWAL_ATTEMPTS as usize);
    for i in 0..WITHDRAWAL_ATTEMPTS {
        let client = Arc::clone(&client);
        handles.push(tokio::spawn(async move {
            client
                .send_instruction(Instruction::Withdrawal(Withdrawal {
                    client_id: 1,
                    tx_id: 100 + i,
                    amount: dec!(1.0),
                }))
                .await
        }));
    }

    let mut applied = 0;
    let mut rejected_as_insufficient = 0;
    let mut unexpected = Vec::new();

    for handle in handles {
        let result = handle.await.unwrap().unwrap();
        match result.outcome {
            InstructionOutcome::Applied => applied += 1,
            InstructionOutcome::Rejected {
                reason: RejectionReason::InsufficientFunds,
            } => rejected_as_insufficient += 1,
            other => unexpected.push(other),
        }
    }

    assert!(unexpected.is_empty(), "unexpected outcomes: {unexpected:?}");
    // Exactly 10 of the 20 attempts can be satisfied by the 10.0 on
    // deposit — never more (no overdraft), never fewer (no funds left
    // stranded that should have been withdrawable).
    assert_eq!(applied, 10);
    assert_eq!(rejected_as_insufficient, 10);

    let final_result = client
        .send_instruction(Instruction::Deposit(Deposit {
            client_id: 1,
            tx_id: 999,
            amount: dec!(0),
        }))
        .await
        .unwrap();
    assert_eq!(final_result.account.total, dec!(0));
    assert_eq!(final_result.account.available, dec!(0));
}

/// Concurrent activity on one client's account must not leak into or
/// interfere with another client's — each producer here only ever
/// touches its own client id, run alongside all the others at once.
#[tokio::test]
async fn concurrent_activity_on_different_clients_does_not_interfere() {
    // These client IDs are spread over four shards, exercising the real
    // routing path rather than the legacy single-engine startup path.
    let client = Arc::new(spawn_sharded_engine(4));
    const CLIENTS: u16 = 10;

    let mut handles = Vec::with_capacity(CLIENTS as usize);
    for client_id in 1..=CLIENTS {
        let client = Arc::clone(&client);
        handles.push(tokio::spawn(async move {
            client
                .send_instruction(Instruction::Deposit(Deposit {
                    client_id,
                    tx_id: u32::from(client_id),
                    amount: Decimal::from(client_id),
                }))
                .await
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    for client_id in 1..=CLIENTS {
        let result = client
            .send_instruction(Instruction::Deposit(Deposit {
                client_id,
                tx_id: u32::from(client_id) + 1000,
                amount: dec!(0),
            }))
            .await
            .unwrap();
        assert_eq!(result.account.total, Decimal::from(client_id));
    }
}

/// A deposit lifecycle is entirely client-scoped: once a deposit reaches its
/// client's shard, dispute and resolve messages for that client must route to
/// the same shard and find its active transaction record there.
#[tokio::test]
async fn dispute_lifecycle_is_preserved_with_sharded_routing() {
    let client = spawn_sharded_engine(4);

    for instruction in [
        Instruction::Deposit(Deposit {
            client_id: 3,
            tx_id: 1,
            amount: dec!(5.0),
        }),
        Instruction::Dispute(crate::domain::Dispute {
            client_id: 3,
            tx_id: 1,
        }),
        Instruction::Resolve(crate::domain::Resolve {
            client_id: 3,
            tx_id: 1,
        }),
    ] {
        assert_eq!(
            client.send_instruction(instruction).await.unwrap().outcome,
            InstructionOutcome::Applied
        );
    }

    let final_state = client
        .send_instruction(Instruction::Withdrawal(Withdrawal {
            client_id: 3,
            tx_id: 2,
            amount: dec!(5.0),
        }))
        .await
        .unwrap();

    assert_eq!(final_state.outcome, InstructionOutcome::Applied);
    assert_eq!(final_state.account.available, dec!(0));
    assert_eq!(final_state.account.held, dec!(0));
    assert_eq!(final_state.account.total, dec!(0));
}
