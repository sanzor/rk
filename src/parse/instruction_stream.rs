use std::error::Error;
use std::fmt;

use tokio::io::AsyncRead;
use tokio::sync::mpsc;
use tokio_util::io::SyncIoBridge;

use crate::csv_row::CsvRow;
use crate::domain::Instruction;
use crate::parse::instruction_parser::parse_instruction;
use crate::parse_error::ParseError;

const STREAM_CHANNEL_CAPACITY: usize = 1024;

/// Why one row of an instruction stream didn't become an [`Instruction`] —
/// either the bytes weren't a well-formed CSV row, or they were but didn't
/// describe a valid instruction (see [`ParseError`]).
#[derive(Debug)]
pub enum StreamError {
    Csv(csv::Error),
    Row(ParseError),
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamError::Csv(error) => write!(f, "csv error: {error}"),
            StreamError::Row(error) => write!(f, "{error}"),
        }
    }
}

impl Error for StreamError {}

/// Parses `reader` as a CSV instruction stream — `type, client, tx, amount`
/// rows — and hands back each row's outcome as soon as it's parsed, never
/// buffering the whole input in memory.
///
/// `reader` only needs to be a canonical [`AsyncRead`]: a `tokio::fs::File`,
/// a TCP socket, or an adapted HTTP request body all work identically, so
/// this same function can back both a CLI file argument and a server
/// handler's request stream — it has no idea which one it's talking to,
/// and no idea an engine client exists. Sending the resulting instructions
/// on to an engine is entirely the caller's job.
///
/// The `csv` crate itself is synchronous, so parsing runs on a blocking
/// thread; `reader` is bridged onto it with [`SyncIoBridge`] so the caller
/// never blocks its own async task waiting on it.
pub fn instruction_stream<R>(reader: R) -> mpsc::Receiver<Result<Instruction, StreamError>>
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
                Ok(row) => parse_instruction(row).map_err(StreamError::Row),
                Err(error) => Err(StreamError::Csv(error)),
            };

            if sender.blocking_send(outcome).is_err() {
                // The receiver was dropped — nobody's listening anymore.
                break;
            }
        }
    });

    receiver
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_csv(name: &str, contents: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("cli_test_{}_{name}.csv", std::process::id()));
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[tokio::test]
    async fn streams_valid_rows_and_skips_bad_ones() {
        let path = temp_csv(
            "stream_mixed",
            "type, client, tx, amount\n\
             deposit, 1, 1, 1.0\n\
             bogus, 2, 2, 1.0\n\
             deposit, 3, 3, notanumber\n\
             withdrawal, 1, 4, 0.5\n",
        );

        let file = tokio::fs::File::open(&path).await.unwrap();
        let mut stream = instruction_stream(file);

        let mut results = Vec::new();
        while let Some(item) = stream.recv().await {
            results.push(item);
        }

        std::fs::remove_file(&path).ok();

        assert_eq!(results.len(), 4);
        assert!(results[0].is_ok());
        assert!(matches!(
            &results[1],
            Err(StreamError::Row(ParseError::UnknownType(kind))) if kind == "bogus"
        ));
        assert!(matches!(&results[2], Err(StreamError::Csv(_))));
        assert!(results[3].is_ok());
    }
}
