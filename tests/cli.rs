//! Black-box tests of the actual compiled `cli` binary — invoked exactly
//! as the brief describes (`cli <path>`, output read from stdout), against
//! real files on disk in `tests/fixtures/`. This is deliberately separate
//! from the inline `#[cfg(test)]` unit tests elsewhere in the crate: those
//! prove the internal logic is correct, but only this proves the shipped
//! executable itself — the thing an automated grader actually runs — reads
//! a file, produces the right CSV on stdout, and exits successfully.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Runs the compiled binary against a fixture file and returns its stdout.
/// Fails the test if the process doesn't exit successfully.
fn run_cli(fixture: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);

    let output = Command::new(env!("CARGO_BIN_EXE_cli"))
        .arg(&path)
        .output()
        .expect("failed to run the cli binary");

    assert!(
        output.status.success(),
        "cli exited with {:?} for {fixture}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout was not valid utf-8")
}

/// One row of the CLI's `client,available,held,total,locked` output.
#[derive(Debug, PartialEq)]
struct AccountRow {
    available: String,
    held: String,
    total: String,
    locked: String,
}

/// Parses the CLI's CSV output into a map keyed by client id, so
/// assertions don't depend on row order — the engine's account map has no
/// defined iteration order, and the brief itself doesn't require one.
fn parse_accounts(csv_output: &str) -> HashMap<u16, AccountRow> {
    let mut reader = csv::Reader::from_reader(csv_output.as_bytes());
    let mut accounts = HashMap::new();

    for record in reader.records() {
        let record = record.expect("malformed CLI output");
        let client: u16 = record[0].parse().expect("non-numeric client id in output");

        accounts.insert(
            client,
            AccountRow {
                available: record[1].to_string(),
                held: record[2].to_string(),
                total: record[3].to_string(),
                locked: record[4].to_string(),
            },
        );
    }

    accounts
}

#[test]
fn basic_deposits_and_withdrawals() {
    let accounts = parse_accounts(&run_cli("basic.csv"));

    assert_eq!(accounts.len(), 2);
    assert_eq!(
        accounts[&1],
        AccountRow {
            available: "1.5".into(),
            held: "0".into(),
            total: "1.5".into(),
            locked: "false".into(),
        }
    );
    assert_eq!(
        accounts[&2],
        AccountRow {
            available: "2".into(),
            held: "0".into(),
            total: "2".into(),
            locked: "false".into(),
        }
    );
}

#[test]
fn dispute_resolve_and_chargeback_lifecycle() {
    let accounts = parse_accounts(&run_cli("dispute_lifecycle.csv"));

    assert_eq!(accounts.len(), 2);

    // Disputed then resolved: back to normal, nothing held.
    assert_eq!(
        accounts[&1],
        AccountRow {
            available: "5".into(),
            held: "0".into(),
            total: "5".into(),
            locked: "false".into(),
        }
    );

    // Disputed then charged back: funds reversed, account frozen.
    assert_eq!(
        accounts[&2],
        AccountRow {
            available: "0".into(),
            held: "0".into(),
            total: "0".into(),
            locked: "true".into(),
        }
    );
}

#[test]
fn malformed_rows_are_skipped_not_fatal() {
    let accounts = parse_accounts(&run_cli("malformed_rows.csv"));

    // Only the valid row ever reached the engine, so only client 1 shows
    // up — the bogus type and the non-numeric amount never became
    // instructions at all, and the run still exits successfully.
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[&1].total, "1");
}

#[test]
fn empty_file_produces_header_only_output() {
    let output = run_cli("empty.csv");
    assert_eq!(output.trim(), "client,available,held,total,locked");
}

#[test]
fn cargo_run_with_redirected_stdout_matches_the_brief() {
    // The other tests here invoke the already-built binary directly via
    // `CARGO_BIN_EXE_cli` — fast and precise, but it skips over `cargo
    // run` itself. This test exercises the literal command the brief
    // documents, `cargo run -- transactions.csv > accounts.csv`, via a
    // real shell, which proves two things the other tests can't:
    //  - `cargo run` still resolves to exactly one binary (this workspace
    //    used to fail with "could not determine which binary to run"
    //    before it was collapsed into a single crate — nothing else here
    //    would catch a regression back to that).
    //  - Cargo's own build/status messages ("Compiling...", "Finished...",
    //    "Running...") land on stderr, not stdout, so shell redirection
    //    captures a clean CSV and nothing else.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let output_path =
        std::env::temp_dir().join(format!("cli_cargo_run_test_{}.csv", std::process::id()));

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cargo run -- tests/fixtures/basic.csv > {}",
            output_path.display()
        ))
        .current_dir(manifest_dir)
        .status()
        .expect("failed to run `cargo run` via a shell");

    assert!(status.success(), "cargo run exited with {status:?}");

    let output =
        std::fs::read_to_string(&output_path).expect("failed to read the redirected output");
    std::fs::remove_file(&output_path).ok();

    let accounts = parse_accounts(&output);

    assert_eq!(accounts.len(), 2);
    assert_eq!(
        accounts[&1],
        AccountRow {
            available: "1.5".into(),
            held: "0".into(),
            total: "1.5".into(),
            locked: "false".into(),
        }
    );
    assert_eq!(
        accounts[&2],
        AccountRow {
            available: "2".into(),
            held: "0".into(),
            total: "2".into(),
            locked: "false".into(),
        }
    );
}

#[test]
fn many_transactions_across_many_clients() {
    // 100 clients, 50 deposits of 1.0 each — big enough to exercise the
    // streaming path over a single request/response file, not just a
    // handful of rows.
    let accounts = parse_accounts(&run_cli("many_transactions.csv"));

    assert_eq!(accounts.len(), 100);
    for client in 1..=100u16 {
        assert_eq!(
            accounts[&client],
            AccountRow {
                available: "50".into(),
                held: "0".into(),
                total: "50".into(),
                locked: "false".into(),
            },
            "client {client} did not end with the expected balance"
        );
    }
}
