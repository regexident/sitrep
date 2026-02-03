#![warn(missing_docs)]

//! Frontend-agnostic progress reporting.

mod error;
mod event;
mod generation;
mod observer;
mod priority;
mod progress;
mod report;
mod task;

pub use self::{
    error::ControlError,
    event::{DetachmentEvent, Event, MessageEvent, UpdateEvent},
    generation::Generation,
    observer::{NopObserver, StdMpscObserver},
    priority::PriorityLevel,
    progress::{Controller, DetachedObserver, Observer, Progress, ProgressId, Reporter},
    report::Report,
    task::{State, Task},
};

#[cfg(any(test, feature = "test-utils"))]
pub use self::progress::test_utils;
