#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::collections::HashMap;
use std::io;
use std::process::ExitCode;

use cli::account_row::AccountRow;
use cli::domain::{Account, ClientId};
use cli::engine_core::{
    EngineClientError, EngineClientProvider, EngineHandle, EngineParams, InMemoryAccountProvider,
    InMemoryTransactionIdRegistry, InMemoryTransactionLedger,
};
use cli::parse::instruction_stream;

#[cfg(test)]
mod process_file_tests;
struct AccountStatusReport {
    pub accounts: Vec<Account>,
}
#[tokio::main]
async fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: cli <transactions.csv>");
        return ExitCode::FAILURE;
    };

    let instructions_csv_file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(error) => {
            eprintln!("error: could not read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };

    const ENGINE_SHARD_COUNT: usize = 64;
    let transaction_ids = InMemoryTransactionIdRegistry::default();
    let engine_handle = EngineHandle::start_sharded(ENGINE_SHARD_COUNT, move || EngineParams {
        accounts: InMemoryAccountProvider::default(),
        transactions: InMemoryTransactionLedger::default(),
        transaction_ids: transaction_ids.clone(),
    });

    let engine_client = engine_handle.client_factory().get_client();

    let report = match stream_instructions_file(instructions_csv_file, &engine_client).await {
        Ok(report) => report,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    write_accounts_report(report)
}

/// Writes `report`'s accounts to stdout as CSV, in the brief's
/// `client,available,held,total,locked` shape.
///
/// The header is written explicitly, up front, rather than left to
/// `csv::Writer`'s usual auto-inference from the first serialized record —
/// with zero accounts there is no first record, and auto-inference would
/// silently emit nothing at all instead of a header-only CSV.
fn write_accounts_report(report: AccountStatusReport) -> ExitCode {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(io::stdout());

    if writer
        .write_record(["client", "available", "held", "total", "locked"])
        .is_err()
    {
        eprintln!("failed writing output");
        return ExitCode::FAILURE;
    }

    for account in report.accounts {
        if writer.serialize(AccountRow::from(account)).is_err() {
            eprintln!("failed writing output");
            return ExitCode::FAILURE;
        }
    }

    if writer.flush().is_err() {
        eprintln!("failed flushing output");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Streams `file` through `client`, and returns the final state of every
/// client account that appeared anywhere in it. The account map starts
/// empty — an account only ever exists because some instruction touched
/// it.
///
/// Rows that can't be parsed are skipped with a warning on stderr rather
/// than aborting the whole run — one bad line in a large file shouldn't
/// discard everything already processed. The engine becoming unavailable
/// is different: that's an infra failure, not a bad row, so it's fatal.
async fn stream_instructions_file(
    file: tokio::fs::File,
    client: &impl EngineClientProvider,
) -> Result<AccountStatusReport, EngineClientError> {
    let mut rows = instruction_stream(file);
    let mut accounts: HashMap<ClientId, Account> = HashMap::new();

    while let Some(row) = rows.recv().await {
        let instruction = match row {
            Ok(instruction) => instruction,
            Err(error) => {
                eprintln!("skipping row: {error}");
                continue;
            }
        };

        let result = client.send_instruction(instruction).await?;
        accounts.insert(result.account.client_id, result.account);
    }

    Ok(AccountStatusReport {
        accounts: accounts.into_values().collect(),
    })
}
