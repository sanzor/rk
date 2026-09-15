/// Identifies a transaction. Unique across all clients and instruction
/// kinds — a deposit and a withdrawal never share a `tx_id`.
///
/// A type alias (not a newtype) so the representation can change later
/// without touching call sites beyond this file.
pub type TransactionId = u32;
