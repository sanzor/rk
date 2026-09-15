use std::error::Error;
use std::fmt;

use tokio::sync::oneshot::error::RecvError;

/// The engine task has stopped (its channel is closed), so a submitted
/// instruction has no one left to process it.
///
/// Unlike a [`RejectionReason`](crate::domain::RejectionReason) — an
/// expected business outcome — this is a genuine infrastructure failure:
/// the process is in a state the caller can't route around.
#[derive(Debug)]
pub enum EngineClientError {
    /// The request never reached the engine at all — its queue could not
    /// accept it because the receiving end is already gone, meaning the
    /// engine's background task has already ended. Carries the sending
    /// channel's own error message.
    SendFailed(String),
    /// The request reached the engine, but its reply channel was dropped
    /// before a reply arrived — typically because the engine's background
    /// task ended (or panicked) before it could respond.
    ReplyLost(RecvError),
}

impl fmt::Display for EngineClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineClientError::SendFailed(message) => {
                write!(f, "the engine is no longer running: {message}")
            }
            EngineClientError::ReplyLost(error) => {
                write!(f, "the engine is no longer running: {error}")
            }
        }
    }
}

impl Error for EngineClientError {}
