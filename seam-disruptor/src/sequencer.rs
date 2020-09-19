use crate::sequence::Sequence;
use crate::sequence_group::SequenceGroup;
use async_std::sync::Arc;
use crossbeam_epoch::{pin, Atomic, Shared};
use std::sync::atomic::Ordering;
use std::time::Duration;

pub struct Sequencer {
    cursor: Sequence,
    gating_sequence_cache: Arc<Sequence>,
    gating_sequences: SequenceGroup,
    buffer_size: u64,
}

impl Sequencer {
    pub fn new(buffer_size: u64) -> Self {
        Self {
            cursor: Sequence::with_value(0),
            gating_sequence_cache: Arc::new(Sequence::with_value(0)),
            gating_sequences: SequenceGroup::new(),
            buffer_size,
        }
    }

    pub(crate) fn register_gating_sequence(&self, sequence: Arc<Sequence>) {
        self.gating_sequences.add(sequence);
    }

    pub async fn next(&self) -> u64 {
        self.next_from(1)
            .await
            .expect("sequencer could not get next sequence number from sequence 1")
    }

    pub async fn next_from(&self, n: u64) -> anyhow::Result<u64> {
        if n < 1 || n > self.buffer_size {
            return Err(anyhow::Error::msg("n must be > 0 and < buffer_size"));
        }

        loop {
            let current = self.cursor.get();
            let next = current + n;

            let wrap_point = next - self.buffer_size;
            let cached_gating_sequence = self.gating_sequence_cache.get();

            if wrap_point > cached_gating_sequence || cached_gating_sequence > current {
                let gating_sequence = self.gating_sequences.minimum_sequence(current);

                if wrap_point > gating_sequence {
                    async_std::task::sleep(Duration::from_nanos(1)).await;
                    continue;
                }

                self.gating_sequence_cache.set(gating_sequence);
            } else if self.cursor.compare_and_swap(current, next) {
                break Ok(next);
            }
        }
    }
}
