/// Identifies a client account.
///
/// A type alias (not a newtype) so the representation can change later
/// without touching call sites beyond this file.
pub type ClientId = u16;
