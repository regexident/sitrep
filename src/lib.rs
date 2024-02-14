#![warn(missing_docs)]

//! Frontend-agnostic progress reporting.

mod progress;
mod task;

pub use self::{
    progress::ProgressId,
    task::{Generation, Task},
};
