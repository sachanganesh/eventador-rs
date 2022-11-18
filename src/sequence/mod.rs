pub(crate) mod sequence_group;
pub(crate) mod sequencer;

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Sequence {
    value: AtomicU64,
}

impl Sequence {
    pub fn with_value(initial_value: u64) -> Self {
        Self {
            value: AtomicU64::from(initial_value),
        }
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Acquire)
    }

    pub fn set(&self, new_value: u64) -> u64 {
        self.value.swap(new_value, Ordering::Release)
    }

    pub fn compare_exchange(&self, expected: u64, new_value: u64) -> bool {
        self.value
            .compare_exchange(expected, new_value, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    pub fn increment(&self) -> u64 {
        self.value.fetch_add(1, Ordering::Release)
    }

    pub fn minimum(&self, other: u64) -> u64 {
        let value = self.get();

        if value < other {
            value
        } else {
            other
        }
    }
}

impl Hash for Sequence {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl Ord for Sequence {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

impl PartialOrd for Sequence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.get().cmp(&other.get()))
    }
}

impl PartialEq for Sequence {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for Sequence {}
