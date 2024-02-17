use std::sync::mpsc::Sender;

use crate::{Event, Observer};

/// Implementation of `Observer` based on `std::sync::mpsc::Sender`.
#[derive(Clone, Debug)]
pub struct StdMpscObserver {
    /// The sending-half of std's channel type.
    pub sender: Sender<Event>,
}

impl From<Sender<Event>> for StdMpscObserver {
    fn from(sender: Sender<Event>) -> Self {
        Self { sender }
    }
}

impl From<StdMpscObserver> for Sender<Event> {
    fn from(observer: StdMpscObserver) -> Self {
        observer.sender
    }
}

impl Observer for StdMpscObserver {
    fn observe(&self, event: Event) {
        let _ = self.sender.send(event);
    }
}

unsafe impl Send for StdMpscObserver where Event: Send {}

unsafe impl Sync for StdMpscObserver where Event: Send {}
