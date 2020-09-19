use crate::sequence::Sequence;
use async_std::sync::Arc;
use crossbeam_epoch::{pin, Atomic, Owned};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct SequenceGroup {
    sequences: Atomic<*mut Arc<Sequence>>,
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

    unsafe fn to_vec(&self) -> Option<Vec<Arc<Sequence>>> {
        let guard = pin();

        let current_ptr = self.sequences.load(Ordering::Acquire, &guard);

        let size = self.size.load(Ordering::Acquire);
        let capacity = self.capacity.load(Ordering::Acquire);

        if let Some(ptr) = current_ptr.as_ref() {
            let p = *ptr;
            Some(Vec::from_raw_parts(p, size, capacity))
        } else {
            None
        }
    }

    pub fn add(&self, sequence: Arc<Sequence>) {
        let guard = pin();

        loop {
            let current = self.sequences.load(Ordering::Acquire, &guard);
            let size = self.size.load(Ordering::Acquire);
            let capacity = self.capacity.load(Ordering::Acquire);
            println!("a!");

            let mut new_sequences: Vec<Arc<Sequence>> = if current.is_null() {
                Vec::new()
            } else {
                let p: *mut Arc<Sequence> = unsafe { *current.as_raw() };
                unsafe { Vec::from_raw_parts(p, size, capacity) }
            };
            new_sequences.push(sequence.clone());
            println!("b!");

            let new_size = new_sequences.len();
            let new_cap = new_sequences.capacity();
            let owned_new_sequences = Owned::new(new_sequences.as_mut_ptr());

            match self.sequences.compare_and_set(
                current,
                owned_new_sequences,
                Ordering::AcqRel,
                &guard,
            ) {
                Ok(_) => {
                    unsafe {
                        guard.defer_destroy(current);
                    }

                    self.size.store(new_size, Ordering::Release);
                    self.capacity.store(new_cap, Ordering::Release);
                    println!("yo!");

                    break;
                }

                Err(cas_err) => {
                    // owned_new_sequences = cas_err.new;
                }
            }
        }
    }

    pub fn remove(&self, sequence: Arc<Sequence>) {
        unimplemented!()
    }

    pub fn get_minimum_sequence(&self, minimum: u64) -> Option<u64> {
        let mut minimum = minimum;

        if let Some(sequences) = unsafe { self.to_vec() } {
            for sequence in sequences {
                let value = sequence.get();
                minimum = std::cmp::min(minimum, value);
            }

            return Some(minimum);
        }

        return None;
    }
}

#[cfg(test)]
mod tests {
    use crate::sequence_group::*;

    #[test]
    fn init_is_none() {
        let sg = SequenceGroup::new();
        assert!(unsafe { sg.to_vec().is_none() })
    }

    #[test]
    fn updates_atomically() {
        let sg = SequenceGroup::new();

        let s1 = Arc::new(Sequence::with_value(1));
        sg.add(s1);

        // let mut v = unsafe { sg.to_vec() };
        // assert!(v.is_some());
        // assert_eq!(v.unwrap().len(), 1);

        // let s2 = Arc::new(Sequence::with_value(5));
        // sg.add(s2);
        //
        // v = unsafe { sg.to_vec() };
        // assert!(v.is_some());
        // assert_eq!(v.unwrap().len(), 1);
    }
}