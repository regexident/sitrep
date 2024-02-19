//! A progress event.

use std::borrow::Cow;

use crate::{PriorityLevel, ProgressId};

/// A progress event.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Event {
    /// A progress had its task updated.
    Update(UpdateEvent),
    /// A progress has posted a message.
    Message(MessageEvent),
    /// A progress has been removed.
    Detachment(DetachmentEvent),
    /// The generation counter has overflown.
    GenerationOverflow,
}

/// A update event.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct UpdateEvent {
    /// The associated progress' identifier.
    pub id: ProgressId,
}

/// A message event.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct MessageEvent {
    /// The associated progress' identifier.
    pub id: ProgressId,
    /// The posted message.
    pub message: Cow<'static, str>,
    /// The message's priority level.
    pub priority: PriorityLevel,
}

/// A update event.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct DetachmentEvent {
    /// The associated progress' identifier.
    pub id: ProgressId,
}
