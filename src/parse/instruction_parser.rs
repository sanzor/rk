use crate::csv_row::CsvRow;
use crate::domain::{ChargeBack, Deposit, Dispute, Instruction, Resolve, Withdrawal};
use crate::parse_error::ParseError;

/// Converts one parsed CSV row into the [`Instruction`] it describes.
pub fn parse_instruction(row: CsvRow) -> Result<Instruction, ParseError> {
    match row.kind.trim().to_lowercase().as_str() {
        "deposit" => Ok(Instruction::Deposit(Deposit {
            client_id: row.client,
            tx_id: row.tx,
            amount: row.amount.ok_or(ParseError::MissingAmount)?,
        })),
        "withdrawal" => Ok(Instruction::Withdrawal(Withdrawal {
            client_id: row.client,
            tx_id: row.tx,
            amount: row.amount.ok_or(ParseError::MissingAmount)?,
        })),
        "dispute" => Ok(Instruction::Dispute(Dispute {
            client_id: row.client,
            tx_id: row.tx,
        })),
        "resolve" => Ok(Instruction::Resolve(Resolve {
            client_id: row.client,
            tx_id: row.tx,
        })),
        "chargeback" => Ok(Instruction::ChargeBack(ChargeBack {
            client_id: row.client,
            tx_id: row.tx,
        })),
        other => Err(ParseError::UnknownType(other.to_string())),
    }
}
