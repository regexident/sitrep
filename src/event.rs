//! A progress event.

use crate::{PriorityLevel, ProgressId};

/// A progress event.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Event {
    /// A progress event.
    Progress(ProgressEvent),
    /// The generation counter has overflown.
    GenerationOverflow,
}

/// A progress event.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ProgressEvent {
    /// The associated progress' identifier.
    pub id: ProgressId,
    /// The event kind.
    pub kind: ProgressEventKind,
}

/// A progress event's kind.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ProgressEventKind {
    /// A progress had its task updated.
    Update,
    /// A progress has posted a message.
    Message(MessageEvent),
    /// A progress has been removed.
    Removed,
}

/// A message event.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct MessageEvent {
    /// The posted message.
    pub message: String,
    /// The message's priority level.
    pub level: PriorityLevel,
}
