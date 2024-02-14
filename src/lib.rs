#![warn(missing_docs)]

//! Frontend-agnostic progress reporting.

mod progress;
mod report;
mod task;

pub use self::{
    progress::ProgressId,
    report::Report,
    task::{Generation, Task},
};
