use std::any::TypeId;
use async_std::sync::Arc;
use crate::sequencer::Sequencer;
use async_channel::{Sender, Receiver};

pub struct Subscriber<T> where T: Send + Sync {
    type_id: TypeId,
    sequence: u64,
    barrier: Arc<Sequencer>,
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T> Subscriber<T> where T: 'static + Send + Sync {
    pub fn new(sequence: u64, sequencer: Arc<Sequencer>, (sender, receiver): (Sender<T>, Receiver<T>)) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            sequence,
            barrier: sequencer,
            sender,
            receiver,
        }
    }

    // pub async fn recv(&self) -> T {
    // }
}