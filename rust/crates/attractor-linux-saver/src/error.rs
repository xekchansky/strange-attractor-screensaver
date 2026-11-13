//! The error both Linux backends return.

use std::error::Error;
use std::fmt;

use attractor_gpu::GpuError;

use crate::session::SessionError;

/// A failure while opening a display, creating a window or running a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellError {
    message: String,
}

impl ShellError {
    /// Build an error with a human-readable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ShellError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ShellError {}

impl From<GpuError> for ShellError {
    fn from(error: GpuError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<SessionError> for ShellError {
    fn from(error: SessionError) -> Self {
        Self::new(error.to_string())
    }
}
