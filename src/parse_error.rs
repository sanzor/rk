use std::error::Error;
use std::fmt;

/// Why a [`CsvRow`](crate::csv_row::CsvRow) couldn't become an
/// [`Instruction`](crate::domain::Instruction). Rows that fail this way
/// are skipped, not fatal — see [`run_file`](crate::run::run_file).
#[derive(Debug)]
pub enum ParseError {
    UnknownType(String),
    MissingAmount,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnknownType(kind) => write!(f, "unknown instruction type {kind:?}"),
            ParseError::MissingAmount => {
                f.write_str("a deposit/withdrawal row is missing its amount")
            }
        }
    }
}

impl Error for ParseError {}
