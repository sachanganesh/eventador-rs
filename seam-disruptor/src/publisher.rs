use crate::ring_buffer::RingBuffer;
use crate::sequencer::Sequencer;
use async_std::sync::Arc;

pub struct Publisher<'a, T> {
    ring: &'a RingBuffer,
}

impl<'a, T> Publisher<T> where T: 'static + Send {
    pub async fn send<T>(&self, msg: T) {
        self.ring.publish(msg).await
    }
}