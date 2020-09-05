use std::any::{TypeId, Any};

pub type EventData = dyn 'static + Any + Send + Sync;

pub struct Event {
    sequence: u64,
    type_id: TypeId,
    data: EventData,
}

impl Event {
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id.clone()
    }

    pub fn data<T: 'static + Clone>(&self) -> Option<T> {
        if TypeId::of::<T>() == self.type_id {
            if let Some(data) = self.data.downcast_ref::<T>() {
                return Some(data.clone())
            }
        }

        return None
    }
}