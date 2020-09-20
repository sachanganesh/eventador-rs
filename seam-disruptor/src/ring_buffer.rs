use crate::event::{EventEnvelope, EventRead};
use crate::sequence::Sequence;
use crate::sequencer::Sequencer;
use crate::subscriber::Subscriber;
use std::sync::Arc;

pub struct RingBuffer {
    capacity: u64,
    buffer: Vec<Arc<EventEnvelope>>,
    sequencer: Sequencer,
}

impl RingBuffer {
    pub fn new(capacity: u64) -> anyhow::Result<Self> {
        if capacity > 1 && capacity.is_power_of_two() {
            let sequencer = Sequencer::new(capacity);

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

    pub fn next(&self) -> u64 {
        self.sequencer.next()
    }

    fn idx_from_sequence(&self, sequence: u64) -> usize {
        (sequence & (self.capacity - 1)) as usize
    }

    pub fn get<'a, T: 'static>(&self, sequence: u64) -> Option<EventRead<'a, T>> {
        let idx = self.idx_from_sequence(sequence);

        if let Some(event) = self.buffer.get(idx) {
            let e = event.clone();

            loop {
                if sequence == e.sequence() {
                    return unsafe { e.read() };
                }
            }
        }

        return None;
    }

    pub fn publish<T: 'static + Send>(&self, message: T) {
        let sequence = self.next();
        let idx = self.idx_from_sequence(sequence);
        if let Some(event_store) = self.buffer.get(idx).clone() {
            event_store.overwrite::<T>(sequence, message);
        }
    }

    pub fn subscribe<'a, T: 'static + Send>(&'a self) -> Subscriber<'a, T> {
        let sequence = Arc::new(Sequence::with_value(self.sequencer.get()));
        self.sequencer.register_gating_sequence(sequence.clone());
        Subscriber::new(&self, sequence)
    }
}

unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new(256).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::ring_buffer::RingBuffer;
    use std::sync::Arc;

    #[test]
    fn error_if_not_power_of_two() {
        assert!(RingBuffer::new(3).is_err());
    }

    #[test]
    fn success_if_power_of_two() {
        assert!(RingBuffer::new(16).is_ok());
    }

    #[test]
    fn publish_and_subscribe() {
        let rb_res = RingBuffer::new(4);
        assert!(rb_res.is_ok());

        let rb = Arc::new(rb_res.unwrap());

        let sub = rb.subscribe::<usize>();
        assert_eq!(0, sub.sequence()); // @todo double check if it should be this way

        let mut i: usize = 1234;
        rb.publish(i);

        let mut msg = sub.recv();
        assert_eq!(i, *msg);

        i += 1;
        let rb2 = rb.clone();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            rb2.publish(i);
        });

        msg = sub.recv();
        assert_eq!(i, *msg);
    }
}
