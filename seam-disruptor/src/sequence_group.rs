use crate::sequence::Sequence;
use async_std::sync::Arc;
use crossbeam_epoch::{pin, Atomic};
use std::sync::atomic::{Ordering, AtomicUsize};

pub struct SequenceGroup {
    sequences: Atomic<*const Arc<Sequence>>,
    size: AtomicUsize,
    capacity: AtomicUsize,
}

impl SequenceGroup {
    pub fn new() -> Self {
        Self {
            sequences: Atomic::null(),
            size: AtomicUsize::from(0),
            capacity: AtomicUsize::from(0),
        }
    }

    pub fn add(&self, sequence: Arc<Sequence>) {
        let guard = pin();

        loop {
            let mut current = self.sequences.load(Ordering::Acquire, &guard);
            let size = self.size.load(Ordering::Acquire);
            let capacity = self.capacity.load(Ordering::Acquire);

            // let mut new_sequences = if current.is_null() {
            //     vec![sequence]
            // } else {
            //     if let Some(prior_sequences) = unsafe { Vec::from_raw_parts(current.deref_mut(), size, capacity) } {
            //         let mut sequences = Vec::from(prior_sequences);
            //         sequences.push(sequence.clone());
            //     } else {
            //         vec![sequence]
            //     }
            // };

            let mut new_sequences: Vec<Arc<Sequence>> = if current.is_null() {
                Vec::new()
            } else {
                let p: *mut Arc<Sequence> = unsafe { &mut **current.mutas_raw() };
                unsafe { Vec::from_raw_parts(p, size, capacity) }
            };
            new_sequences.push(sequence.clone());

            match self
                .sequences
                .compare_and_set(current, new_sequences, Ordering::AcqRel, &guard)
            {
                Ok(_) => {
                    unsafe {
                        guard.defer_destroy(current);
                    }

                    break;
                }
                Err(cas_err) => {
                    new_sequences = &mut Vec::from(cas_err.new);
                }
            }
        }
    }

    pub fn remove(&self, sequence: Arc<Sequence>) {
        unimplemented!()
    }

    pub fn get_minimum_sequence(&self, minimum: u64) -> Option<u64> {
        let mut minimum = minimum;

        let guard = pin();

        if let Some(sequences) = unsafe { self.sequences.load(Ordering::Acquire, &guard).as_ref() }
        {
            for sequence in sequences {
                let value = sequence.get();
                minimum = std::cmp::min(minimum, value);
            }

            return Some(minimum);
        }

        return None;
    }
}
