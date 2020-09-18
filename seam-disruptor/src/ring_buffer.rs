use crate::event::{EventEnvelope, EventRead};
use crate::sequence::Sequence;
use crate::sequencer::Sequencer;
use crate::subscriber::Subscriber;
use async_std::sync::Arc;

pub struct RingBuffer {
    capacity: u64,
    buffer: Vec<Arc<EventEnvelope>>,
    sequencer: Sequencer,
}

impl RingBuffer {
    pub fn new(capacity: u64, sequencer: Sequencer) -> anyhow::Result<Self> {
        if capacity > 1 && capacity.is_power_of_two() {
            let ucapacity = capacity as usize;
            let mut buffer = Vec::with_capacity(ucapacity);
            for i in 0..ucapacity {
                buffer.insert(i, Arc::new(EventEnvelope::new()))
            }

            Ok(Self {
                capacity,
                buffer, // @todo explore cache line optimization with padding
                sequencer,
            })
        } else {
            Err(anyhow::Error::msg("expected capacity as a power of two"))
        }
    }

    pub async fn next(&self) -> u64 {
        self.sequencer.next().await
    }

    fn idx_from_sequence(&self, sequence: u64) -> usize {
        (sequence & (self.capacity - 1)) as usize
    }

    pub fn get<'a, T: 'static>(&self, sequence: u64) -> Option<EventRead<'a, T>> {
        let idx = self.idx_from_sequence(sequence);

        if let Some(event) = self.buffer.get(idx) {
            let e = event.clone();
            return unsafe { e.read() };
        }

        return None;
    }

    pub async fn publish<T: 'static + Send>(&self, message: T) {
        let idx = self.idx_from_sequence(self.next().await);

        if let Some(event_store) = self.buffer.get(idx).clone() {
            event_store.overwrite::<T>(message);
        }
    }

    pub async fn subscribe<'a, T: 'static + Send>(&self) -> Subscriber<'a, T> {
        let sequence = Arc::new(Sequence::with_value(self.sequencer.next().await));
        self.sequencer.register_gating_sequence(sequence.clone());
        Subscriber::new(&self, sequence)
    }
}

#[cfg(test)]
mod tests {
    use crate::ring_buffer::RingBuffer;
    use crate::sequencer::Sequencer;
    use async_std::sync::Arc;

    #[test]
    fn error_if_not_power_of_two() {
        let s = Sequencer::new(0);
        assert!(RingBuffer::new(3, s).is_err());
    }

    #[test]
    fn success_if_power_of_two() {
        let s = Sequencer::new(0);
        assert!(RingBuffer::new(16, s).is_ok());
    }
}
