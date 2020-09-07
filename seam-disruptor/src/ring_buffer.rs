use async_std::sync::Arc;
use crate::sequencer::Sequencer;
use crate::event::EventStore;

pub struct RingBuffer {
    capacity: u64,
    buffer: Vec<Arc<EventStore>>,
    sequencer: Arc<Sequencer>
}

impl RingBuffer {
    pub fn new(capacity: u64, sequencer: Arc<Sequencer>) -> anyhow::Result<Self> {
        if capacity > 1 && capacity.is_power_of_two() {
            let ucapacity = capacity as usize;
            let mut buffer = Vec::with_capacity(ucapacity);
            for i in 0..ucapacity {
                buffer.insert(i, Arc::new(EventStore::new()))
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

    pub fn get(&self, sequence: u64) -> Arc<EventStore> {
        let idx = sequence & (self.capacity - 1);

        if let Some(event) = self.buffer.get(idx as usize) {
            return event.clone()
        }

        unreachable!()
    }

    pub async fn publish<T: 'static + Send + Sync>(&self, message: T) {
        let sequence = self.sequencer.next().await;

        let event_store = self.get(sequence);
        event_store.overwrite(message);
    }
}

#[cfg(test)]
mod tests {
    use async_std::sync::Arc;
    use crate::ring_buffer::RingBuffer;
    use crate::sequencer::Sequencer;

    #[test]
    fn error_if_not_power_of_two() {
        let s = Sequencer::new(0);
        assert!(RingBuffer::new(3, Arc::from(s)).is_err());
    }

    #[test]
    fn success_if_power_of_two() {
        let s = Sequencer::new(0);
        assert!(RingBuffer::new(16, Arc::from(s)).is_ok());
    }
}