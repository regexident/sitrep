#![warn(missing_docs)]

//! Frontend-agnostic progress reporting.

mod event;
mod generation;
mod observer;
mod priority;
mod progress;
mod report;
mod task;

pub use self::{
    event::{DetachmentEvent, Event, MessageEvent, UpdateEvent},
    generation::Generation,
    observer::{NopObserver, StdMpscObserver},
    priority::PriorityLevel,
    progress::{Controller, Observer, Progress, ProgressId, Reporter},
    report::Report,
    task::{State, Task},
};

#[cfg(any(test, feature = "test-utils"))]
pub use self::progress::test_utils;
