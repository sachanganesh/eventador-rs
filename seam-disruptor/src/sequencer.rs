use std::sync::atomic::AtomicU64;

pub struct Sequencer {
    sequence_number: AtomicU64
}

impl Sequencer {
    pub fn new(sequence_number: u64) -> Self {
        Self {
            sequence_number: AtomicU64::new(sequence_number)
        }
    }

    pub fn next(&self) -> u64 {
        unimplemented!()
    }

    pub fn next_from(&self, n: u64) -> u64 {
        unimplemented!()
    }

    pub fn publish(&self, n: u64) {
        unimplemented!()
    }

    pub fn publish_range(&self, lo: u64, hi: u64) {
        unimplemented!()
    }
}