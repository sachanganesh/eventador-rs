pub mod net;

use async_channel::{bounded, unbounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Serialize, Deserialize)]
pub struct StitchMessage {
    pub type_id: u64,
    pub bytes: Vec<u8>,
}

impl StitchMessage {
    pub fn type_id<T: 'static>() -> TypeId {
        TypeId::of::<T>()
    }

    pub fn hash_type_id(tid: TypeId) -> u64 {
        let mut hasher = DefaultHasher::new();
        tid.hash(&mut hasher);
        hasher.finish()
    }

    pub fn hash_type<T: 'static>() -> u64 {
        Self::hash_type_id(Self::type_id::<T>())
    }
}

pub(crate) fn channel_factory<
    T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
>(
    mut bound: Option<usize>,
) -> (Sender<T>, Receiver<T>) {
    if let Some(bound) = bound.take() {
        bounded(bound)
    } else {
        unbounded()
    }
}
