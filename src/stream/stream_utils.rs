use std::error::Error;
use std::fmt;

use tokio::io::AsyncRead;
use tokio::sync::mpsc;
use tokio_util::io::SyncIoBridge;

use crate::csv_row::CsvRow;
use crate::domain::Instruction;
use crate::instruction_row::parse_instruction;
use crate::parse_error::ParseError;

const STREAM_CHANNEL_CAPACITY: usize = 1024;

/// Why one row of an instruction stream didn't become an [`Instruction`] —
/// either the bytes weren't a well-formed CSV row, or they were but didn't
/// describe a valid instruction (see [`ParseError`]).
#[derive(Debug)]
pub enum InstructionStreamError {
    Csv(csv::Error),
    Parse(ParseError),
}

impl fmt::Display for InstructionStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstructionStreamError::Csv(error) => write!(f, "csv error: {error}"),
            InstructionStreamError::Parse(error) => write!(f, "{error}"),
        }
    }
}

impl Error for InstructionStreamError {}

/// Parses `reader` as a CSV instruction stream — `type, client, tx, amount`
/// rows — and hands back each row's outcome as soon as it's parsed, never
/// buffering the whole input in memory.
///
/// `reader` only needs to be a canonical [`AsyncRead`]: a `tokio::fs::File`,
/// a TCP socket, or an adapted HTTP request body all work identically, so
/// this same function can back both a CLI file argument and a server
/// handler's request stream — it has no idea which one it's talking to,
/// and no idea an [`EngineClient`](crate::engine_core::EngineClient)
/// exists.
///
/// The `csv` crate itself is synchronous, so parsing runs on a blocking
/// thread; `reader` is bridged onto it with [`SyncIoBridge`] so the caller
/// never blocks its own async task waiting on it.
pub fn generate_stream<R>(
    reader: R,
) -> InstructionStream
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let (sender, receiver) = mpsc::channel(STREAM_CHANNEL_CAPACITY);

    tokio::task::spawn_blocking(move || {
        let mut csv_reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .flexible(true)
            .from_reader(SyncIoBridge::new(reader));

        for record in csv_reader.deserialize::<CsvRow>() {
            let outcome = match record {
                Ok(row) => parse_instruction(row).map_err(InstructionStreamError::Parse),
                Err(error) => Err(InstructionStreamError::Csv(error)),
            };

            if sender.blocking_send(outcome).is_err() {
                // The receiver was dropped — nobody's listening anymore.
                break;
            }
        }
    });

    InstructionStream{stream:receiver}
}
