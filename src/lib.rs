#![warn(missing_docs)]

//! Frontend-agnostic progress reporting.

mod priority;
mod progress;
mod report;
mod task;

pub use self::{
    priority::PriorityLevel,
    progress::ProgressId,
    report::Report,
    task::{Generation, Task},
};
