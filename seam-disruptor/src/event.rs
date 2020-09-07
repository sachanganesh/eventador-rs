use std::any::{TypeId, Any};
use std::sync::atomic::{AtomicU64, Ordering, AtomicPtr};
use crossbeam_utils::atomic::AtomicCell;

pub struct Event {
    pub type_id: TypeId,
    pub data: Box<dyn Any>
}

pub struct EventStore {
    read: AtomicU64,
    event: AtomicPtr<Event>,
}

impl EventStore {
    pub fn new() -> Self {
        Self {
            read: AtomicU64::new(0),
            event: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    // pub fn with_data<T>(mut data: T) -> Self {
    //     let event_ptr: *mut Event = &mut Box::new(data) as Box<Event>;
    //
    //     Self {
    //         read: AtomicU64::new(0),
    //         type_id: AtomicCell::new(Some(TypeId::of::<T>())),
    //         event: AtomicPtr::new(event_ptr)
    //     }
    // }

    pub fn times_read(&self) -> u64 {
        self.read.load(Ordering::Acquire)
    }

    pub unsafe fn read<T: 'static>(&self) -> Option<&T> {
        if let Some(event) = self.event.load(Ordering::Release).as_ref() {
            if TypeId::of::<T>() == event.type_id {
                if let Some(data) = event.data.downcast_ref::<Box<T>>() {
                    self.read.fetch_add(1, Ordering::Release);
                    return Some(data.as_ref())
                }
            }
        }

        return None
    }

    pub fn overwrite<T: 'static>(&self, data: T) {
        let mut event = Event {
            type_id: TypeId::of::<T>(),
            data: Box::new(data),
        };
        let event_ptr: *mut Event = &mut event;

        let _old_data = self.event.swap(event_ptr, Ordering::AcqRel);
        self.read.store(0, Ordering::Release);
    }
}