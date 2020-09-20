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

    pub fn sequence(&self) -> u64 {
        self.sequence.get()
    }

    pub async fn recv<'b>(&self) -> EventRead<'b, T> {
        loop {
            let sequence = self.sequence.increment();

            if let Some(event) = self.ring.get(sequence).await {
                return event;
            }
        }
    }
}
