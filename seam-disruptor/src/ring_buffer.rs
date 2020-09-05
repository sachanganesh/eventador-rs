use async_channel::{Sender, Receiver, unbounded};
use async_std::sync::Arc;
use crate::sequencer::Sequencer;
use crate::event::Event;
use crate::subscriber::Subscriber;

pub struct RingBuffer {
    capacity: u64,
    buffer: Vec<Arc<Event>>,
    sequencer: Arc<Sequencer>
}

impl RingBuffer {
    pub fn new(capacity: u64, sequencer: Arc<Sequencer>) -> anyhow::Result<Self> {
        if capacity > 1 && capacity.is_power_of_two() {
            Ok(Self {
                capacity,
                buffer: Vec::with_capacity(capacity as usize), // @todo explore cache line optimization with padding
                sequencer,
            })
        } else {
            Err(anyhow::Error::msg("expected capacity as a power of two"))
        }
    }

    pub fn get<T>(&self, sequence: u64) -> Option<Arc<Event>> {
        let idx = sequence & (self.capacity - 1);

        if let Some(event) = self.buffer.get(idx as usize) {
            if event.sequence() == sequence {
                return Some(event.clone())
            }
        }

        return None
    }

    pub fn publish<T: 'static + Send + Sync>(&self, message: T) {
        // @todo insert into data store
        let sequence = self.sequencer.next();
        self.sequencer.publish(sequence)
    }

    pub fn subscribe<T: 'static + Send + Sync>(&self) -> Subscriber<T> {
        Subscriber::new(0, self.sequencer.clone(), unbounded())
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