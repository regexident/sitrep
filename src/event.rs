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
    /// The generation counter has overflowed.
    ///
    /// This event is emitted when the internal generation counter wraps around after
    /// reaching [`usize::MAX`]. Generation counters are used to track changes to progress
    /// tasks and enable efficient delta reporting via [`Reporter::partial_report`].
    ///
    /// # When This Occurs
    ///
    /// Generation overflow is extremely rare in practice, requiring billions of task updates
    /// (2^64 on 64-bit systems, 2^32 on 32-bit systems). In typical applications, this event
    /// will never occur during normal operation.
    ///
    /// # What To Do
    ///
    /// When this event is received, observers should:
    ///
    /// - Log the occurrence for monitoring purposes
    /// - Continue normal operation - the generation counter wraps safely
    /// - Be aware that generation-based comparisons may temporarily be incorrect immediately
    ///   after overflow, but will self-correct as new updates occur
    ///
    /// No action is typically required as the system continues to function correctly after
    /// overflow. The wrapping behavior is intentional and safe.
    ///
    /// [`Reporter::partial_report`]: crate::Reporter::partial_report
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

/// A detachment event.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct DetachmentEvent {
    /// The associated progress' identifier.
    pub id: ProgressId,
}
