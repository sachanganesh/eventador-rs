use crate::event::{EventRead, EventReadLabel};
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use futures::task::{Context, Poll};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

pub struct AsyncSubscriber<'a, T> {
    ring: Arc<RingBuffer>,
    sequence: Arc<Sequence>,
    _marker: std::marker::PhantomData<&'a T>,
}

impl<'a, T> AsyncSubscriber<'a, T>
where
    T: 'static + Send,
{
    pub fn new(ring: Arc<RingBuffer>, sequence: Arc<Sequence>) -> Self {
        Self {
            ring,
            sequence,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn sequence(&self) -> u64 {
        self.sequence.get()
    }

    pub async fn recv(&self) -> Option<EventRead<'a, T>> {
        loop {
            let sequence = self.sequence.get();

            loop {
                match self.ring.get_event(sequence) {
                    EventReadLabel::Irrelevant => {
                        self.sequence.increment();
                        break;
                    }

                    EventReadLabel::Relevant(event) => {
                        self.sequence.increment();
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

impl<'a, T: 'static> Stream for AsyncSubscriber<'a, T> {
    type Item = EventRead<'a, T>;

    // @todo: Needs thorough testing due to failed prior tests
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let sequence = self.sequence.get();

            match self.ring.get_event(sequence) {
                EventReadLabel::Irrelevant => {
                    self.sequence.increment();
                    continue;
                }

                EventReadLabel::Relevant(event) => {
                    self.sequence.increment();
                    return Poll::Ready(Some(event));
                }

                EventReadLabel::Waiting => {
                    return Poll::Pending;
                }
            }
        }
    }
}
