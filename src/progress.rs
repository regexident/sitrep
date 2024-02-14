//! A progress.

use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// A progress' unique identifier.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ProgressId(pub(crate) usize);

impl ProgressId {
    pub(crate) fn new_unique() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_auto_incrementing() {
        let ids: Vec<_> = (0..100).map(|_| ProgressId::new_unique()).collect();

        for window in ids.windows(2) {
            let [prev, next] = window else {
                panic!("expected window of size 2");
            };

            assert_eq!(prev.0 + 1, next.0);
        }
    }
}
