#![warn(missing_docs)]

//! Frontend-agnostic progress reporting.

mod event;
mod priority;
mod progress;
mod report;
mod task;

pub use self::{
    event::{Event, MessageEvent, ProgressEvent, ProgressEventKind},
    priority::PriorityLevel,
    progress::{Observer, Progress, ProgressId, Reporter},
    report::Report,
    task::{Generation, Task},
};
