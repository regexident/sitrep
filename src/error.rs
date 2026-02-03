//! Error types for progress control operations.

use std::fmt;

/// Error type for control operations that cannot be performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlError {
    /// The progress task is not pausable.
    NotPausable,
    /// The progress task is not cancelable.
    NotCancelable,
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPausable => write!(f, "progress task is not pausable"),
            Self::NotCancelable => write!(f, "progress task is not cancelable"),
        }
    }
}

impl std::error::Error for ControlError {}
