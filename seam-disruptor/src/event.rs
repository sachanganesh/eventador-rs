use crossbeam_epoch::{pin, Atomic, Guard, Owned, Shared};
use std::any::{Any, TypeId};
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Event {
    pub type_id: TypeId,
    pub data: Box<dyn Any>,
}

pub struct EventEnvelope {
    read_count: AtomicU64,
    event: Atomic<Event>,
}

pub struct EventRead<'a, T: 'a> {
    _guard: Guard,
    raw: *const T,
    _marker: std::marker::PhantomData<&'a T>,
}

impl<'a, T> Deref for EventRead<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.raw }
    }
}

impl EventEnvelope {
    pub fn new() -> Self {
        Self {
            read_count: AtomicU64::new(0),
            event: Atomic::null(),
        }
    }

    pub fn read_count(&self) -> u64 {
        unsafe { self.read_count.load(Ordering::Acquire) }
    }

    pub fn read<'a, T: 'static>(&self) -> Option<EventRead<'a, T>> {
        let guard = pin();

        if let Some(event) = unsafe { self.event.load(Ordering::Release, &guard).as_ref() } {
            if TypeId::of::<T>() == event.type_id {
                let event_data = unsafe { std::ptr::read(&(*event).data) };

                if let Some(casted_data) = event_data.downcast_ref::<Box<T>>() {
                    self.read_count.fetch_add(1, Ordering::Release);

                    return Some(EventRead {
                        _guard: guard,
                        raw: unsafe { &**casted_data },
                        _marker: std::marker::PhantomData,
                    });
                }
            }
        }

        return None;
    }

    pub fn overwrite<T: 'static>(&self, data: T) {
        let mut event = Owned::new(Event {
            type_id: TypeId::of::<T>(),
            data: Box::new(data),
        });

        let guard = pin();

        loop {
            let current_event = self.event.load(Ordering::Acquire, &guard);

            match self
                .event
                .compare_and_set(current_event, event, Ordering::AcqRel, &guard)
            {
                Ok(_) => {
                    self.read_count.store(0, Ordering::Release);

                    unsafe {
                        guard.defer_destroy(current_event);
                    }

                    break;
                }
                Err(cas_fail) => {
                    event = cas_fail.new;
                }
            }
        }
    }
}
