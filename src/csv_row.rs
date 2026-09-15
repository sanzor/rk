use rust_decimal::Decimal;
use serde::Deserialize;

use crate::domain::{ClientId, TransactionId};

/// One row of the input CSV, exactly as it appears — `type, client, tx,
/// amount`. `amount` is absent for a dispute/resolve/chargeback row, per
/// the brief.
///
/// Deliberately flat rather than deserialized straight into
/// [`Instruction`](crate::domain::Instruction): its internally-tagged
/// shape isn't something the `csv` crate's row-oriented deserialization
/// understands. [`parse_instruction`](crate::instruction_row::parse_instruction)
/// does that conversion instead.
#[derive(Debug, Deserialize)]
pub struct CsvRow {
    #[serde(rename = "type")]
    pub kind: String,
    pub client: ClientId,
    pub tx: TransactionId,
    pub amount: Option<Decimal>,
}
