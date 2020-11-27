use crate::event::{EventRead, EventReadLabel};
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use std::sync::Arc;

pub struct Subscriber<'a, T> {
    ring: &'a RingBuffer,
    sequence: Arc<Sequence>,
    _marker: std::marker::PhantomData<T>,
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

    pub fn recv<'b>(&self) -> Option<EventRead<'b, T>> {
        loop {
            let sequence = self.sequence.increment();

            loop {
                match self.ring.get_event(sequence) {
                    EventReadLabel::Irrelevant => {
                        break;
                    }

                    EventReadLabel::Relevant(event) => {
                        return Some(event);
                    }

                    EventReadLabel::Waiting => {
                        continue;
                    }
                }
            }
        }
    }
}
