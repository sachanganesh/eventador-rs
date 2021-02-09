use crate::event::EventEnvelope;
use crate::sequence::sequencer::Sequencer;
use crate::WaitStrategy;
use crossbeam::utils::CachePadded;
use std::sync::Arc;

pub(crate) type EventWrapper = CachePadded<Arc<EventEnvelope>>;

pub struct RingBuffer {
    capacity: u64,
    buffer: Vec<EventWrapper>,
    sequencer: Sequencer,
    wait_strategy: WaitStrategy,
}

impl RingBuffer {
    pub fn new(capacity: u64, wait_strategy: WaitStrategy) -> anyhow::Result<Self> {
        if capacity > 1 && capacity.is_power_of_two() {
            let sequencer = Sequencer::new(capacity, wait_strategy);

            let ucapacity = capacity as usize;
            let mut buffer = Vec::with_capacity(ucapacity);

            for i in 0..ucapacity {
                buffer.insert(i, CachePadded::new(Arc::new(EventEnvelope::new())))
            }

            Ok(Self {
                capacity,
                buffer,
                sequencer,
                wait_strategy,
            })
        } else {
            Err(anyhow::Error::msg("expected capacity as a power of two"))
        }
    }

    pub(crate) fn sequencer(&self) -> &Sequencer {
        &self.sequencer
    }

    pub(crate) fn wait_strategy(&self) -> WaitStrategy {
        self.wait_strategy
    }

    pub(crate) fn next(&self) -> u64 {
        self.sequencer.next()
    }

    pub(crate) async fn async_next(&self) -> u64 {
        self.sequencer.async_next().await
    }

    pub(crate) fn idx_from_sequence(&self, sequence: u64) -> usize {
        (sequence & (self.capacity - 1)) as usize
    }

    pub(crate) fn get_envelope(&self, sequence: u64) -> Option<EventWrapper> {
        let idx = self.idx_from_sequence(sequence);

        if let Some(envelope) = self.buffer.get(idx).clone() {
            Some(envelope.clone())
        } else {
            None
        }
    }
}

unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new(256, WaitStrategy::AllSubscribers).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::ring_buffer::RingBuffer;
    use crate::WaitStrategy;

    #[test]
    fn error_if_not_power_of_two() {
        assert!(RingBuffer::new(3, WaitStrategy::AllSubscribers).is_err());
    }

    #[test]
    fn success_if_power_of_two() {
        assert!(RingBuffer::new(16, WaitStrategy::AllSubscribers).is_ok());
    }
}
