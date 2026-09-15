#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::csv_row::CsvRow;
    use crate::domain::{Deposit, Dispute, Instruction};
    use crate::parse::instruction_parser::parse_instruction;
    use crate::parse_error::ParseError;

    #[test]
    fn parses_a_deposit() {
        let instruction = parse_instruction(CsvRow {
            kind: "deposit".to_string(),
            client: 1,
            tx: 1,
            amount: Some(dec!(1.5)),
        })
        .unwrap();

        assert_eq!(
            instruction,
            Instruction::Deposit(Deposit {
                client_id: 1,
                tx_id: 1,
                amount: dec!(1.5),
            })
        );
    }

    #[test]
    fn trims_and_lowercases_the_type() {
        let instruction = parse_instruction(CsvRow {
            kind: " Dispute ".to_string(),
            client: 1,
            tx: 1,
            amount: None,
        })
        .unwrap();

        assert_eq!(
            instruction,
            Instruction::Dispute(Dispute {
                client_id: 1,
                tx_id: 1,
            })
        );
    }

    #[test]
    fn rejects_an_unknown_type() {
        let error = parse_instruction(CsvRow {
            kind: "bogus".to_string(),
            client: 1,
            tx: 1,
            amount: None,
        })
        .unwrap_err();

        assert!(matches!(error, ParseError::UnknownType(kind) if kind == "bogus"));
    }

    #[test]
    fn rejects_a_deposit_with_no_amount() {
        let error = parse_instruction(CsvRow {
            kind: "deposit".to_string(),
            client: 1,
            tx: 1,
            amount: None,
        })
        .unwrap_err();

        assert!(matches!(error, ParseError::MissingAmount));
    }

    #[test]
    fn rejects_a_withdrawal_with_no_amount() {
        let error = parse_instruction(CsvRow {
            kind: "withdrawal".to_string(),
            client: 1,
            tx: 1,
            amount: None,
        })
        .unwrap_err();

        assert!(matches!(error, ParseError::MissingAmount));
    }
}
