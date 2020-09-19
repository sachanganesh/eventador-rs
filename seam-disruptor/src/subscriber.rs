use crate::event::EventRead;
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use async_std::sync::Arc;
use async_std::task::JoinHandle;

pub struct Subscriber<'a, T> {
    ring: &'a RingBuffer,
    sequence: Arc<Sequence>,
    _marker: std::marker::PhantomData<T>,
    // task: JoinHandle<()>,
}

impl<'a, T> Subscriber<'a, T>
where
    T: 'static + Send,
{
    pub fn new(ring: &'a RingBuffer, sequence: Arc<Sequence>) -> Self {
        Self {
            ring,
            sequence,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn recv<'b>(&self) -> EventRead<'b, T> {
        loop {
            let sequence = self.sequence.get();
            self.sequence.set(sequence + 1);

            if let Some(event) = self.ring.get(sequence) {
                return event;
            }
        }
    }
}
