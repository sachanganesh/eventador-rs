use crate::sequence::sequence_group::SequenceGroup;
use crate::sequence::Sequence;
use crate::WaitStrategy;
use std::sync::Arc;

pub struct Sequencer {
    cursor: Sequence,
    gating_sequence: Arc<Sequence>,
    ring_capacity: u64,
    wait_strategy: WaitStrategy,
}

impl Sequencer {
    pub fn new(ring_capacity: u64, wait_strategy: WaitStrategy) -> Self {
        Self {
            cursor: Sequence::with_value(0),
            gating_sequence: Arc::new(Sequence::with_value(0)),
            ring_capacity,
            wait_strategy,
        }
    }

    pub(crate) fn register_gating_sequence(&self, sequence: Arc<Sequence>) {
        self.gating_sequence.set(sequence)
    }

    pub fn get(&self) -> u64 {
        self.cursor.get()
    }

    pub fn next(&self) -> u64 {
        self.next_from(1)
            .expect("sequencer could not get next sequence number from sequence 1")
    }

    #[cfg(feature = "async")]
    pub async fn async_next(&self) -> u64 {
        self.async_next_from(1)
            .await
            .expect("sequencer could not get next sequence number from sequence 1")
    }

    pub fn next_from(&self, n: u64) -> anyhow::Result<u64> {
        if n < 1 || n > self.ring_capacity {
            return Err(anyhow::Error::msg("n must be > 0 and < buffer_size"));
        }

        loop {
            std::hint::spin_loop();

            let current: u64 = self.cursor.get();
            let icurrent: i64 = current as i64;
            let next: i64 = (current + n) as i64;

            let wrap_point: i64 = next - self.ring_capacity as i64;
            let cached_gating_sequence: i64 = self.gating_sequence.get() as i64;

            if wrap_point >= cached_gating_sequence || cached_gating_sequence > icurrent {
                let gating_sequence = self.gating_sequences.minimum_sequence(current);

                match self.wait_strategy {
                    WaitStrategy::AllSubscribers => {
                        if wrap_point > gating_sequence as i64 {
                            // @todo refactor or consider using yield_now
                            std::thread::sleep(std::time::Duration::from_micros(100));
                            continue;
                        }
                    }

                    WaitStrategy::NoWait => {
                        if self.cursor.compare_exchange(current, next as u64) {
                            return Ok(next as u64);
                        }
                    }

                    WaitStrategy::WaitForDuration(wait) => {
                        if self.cursor.compare_exchange(current, next as u64) {
                            std::thread::sleep(wait);

                            return Ok(next as u64);
                        }
                    }
                }

                self.gating_sequence.set(gating_sequence);
            } else if self.cursor.compare_exchange(current, next as u64) {
                return Ok(next as u64);
            }
        }
    }

    #[cfg(feature = "async")]
    pub async fn async_next_from(&self, n: u64) -> anyhow::Result<u64> {
        if n < 1 || n > self.ring_capacity {
            return Err(anyhow::Error::msg("n must be > 0 and < buffer_size"));
        }

        loop {
            let current: u64 = self.cursor.get();
            let icurrent: i64 = current as i64;
            let next: i64 = (current + n) as i64;

            let wrap_point: i64 = next - self.ring_capacity as i64;
            let cached_gating_sequence: i64 = self.gating_sequence.get() as i64;

            if wrap_point >= cached_gating_sequence || cached_gating_sequence > icurrent {
                let gating_sequence = self.gating_sequences.minimum_sequence(current);

                match self.wait_strategy {
                    WaitStrategy::AllSubscribers => {
                        if wrap_point > gating_sequence as i64 {
                            async_std::task::sleep(std::time::Duration::from_micros(100)).await;
                            continue;
                        }
                    }

                    WaitStrategy::NoWait => {
                        if self.cursor.compare_exchange(current, next as u64) {
                            return Ok(next as u64);
                        }
                    }

                    WaitStrategy::WaitForDuration(wait) => {
                        if self.cursor.compare_exchange(current, next as u64) {
                            async_std::task::sleep(wait).await;

                            return Ok(next as u64);
                        }
                    }
                }

                self.gating_sequence.set(gating_sequence);
            } else if self.cursor.compare_exchange(current, next as u64) {
                return Ok(next as u64);
            }
        }
    }
}
