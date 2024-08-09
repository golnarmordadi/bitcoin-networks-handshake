// error.rs
use std::io;
use tokio::time::error::Elapsed;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Connection failed: {0:?}")]
    ConnectionFailed(io::Error),
    #[error("Connection timed out")]
    ConnectionTimedOut(Elapsed),
    #[error("Connection lost")]
    ConnectionLost,
    #[error("Sending failed")]
    SendingFailed(io::Error),
    #[error("Invalid address format for {0}")]
    InvalidAddress(String),
}
