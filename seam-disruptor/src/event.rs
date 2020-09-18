use crossbeam_epoch::{pin, Atomic, Guard, Owned};
use std::any::{Any, TypeId};
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct Event {
    pub type_id: TypeId,
    pub data: Box<dyn Any>,
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

pub struct EventEnvelope {
    read_count: AtomicU64,
    event: Atomic<Event>,
}

impl EventEnvelope {
    pub fn new() -> Self {
        Self {
            read_count: AtomicU64::new(0),
            event: Atomic::null(),
        }
    }

    pub fn read_count(&self) -> u64 {
        self.read_count.load(Ordering::Acquire)
    }

    pub unsafe fn read<'a, T: 'static>(&self) -> Option<EventRead<'a, T>> {
        let guard = pin();

        let event = self.event.load(Ordering::Acquire, &guard).as_raw();

        if !event.is_null() && TypeId::of::<T>() == (*event).type_id {
            if let Some(event_data) = (*event).data.downcast_ref() {
                self.read_count.fetch_add(1, Ordering::Release);

                return Some(EventRead {
                    _guard: guard,
                    raw: &*event_data,
                    _marker: std::marker::PhantomData,
                });
            }
        }

        return None;
    }

    pub(crate) fn overwrite<T: 'static>(&self, data: T) {
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
                Err(cas_err) => {
                    event = cas_err.new;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::event::*;

    #[test]
    fn event_read_count_init() {
        let e = EventEnvelope::new();
        assert_eq!(0, e.read_count());
    }

    #[test]
    fn event_read() {
        let e = EventEnvelope::new();
        let r = unsafe { e.read::<String>() };
        assert!(r.is_none());
        assert_eq!(0, e.read_count());
    }

    #[test]
    fn event_overwrite() {
        let e = EventEnvelope::new();
        e.overwrite(String::from("Hello world!"));

        let readable_event = unsafe { e.read::<String>() };
        assert!(readable_event.is_some());

        let read_msg = &*readable_event.unwrap();
        let expected_msg = String::from("Hello world!");

        assert!(expected_msg.eq(read_msg));
        assert_eq!(1, e.read_count());

        e.overwrite(String::from("Bye Felicia!"));

        let another_readable_event = unsafe { e.read::<String>() };
        assert!(another_readable_event.is_some());

        let another_read_msg = &*another_readable_event.unwrap();
        let another_expected_msg = String::from("Bye Felicia!");

        assert!(another_expected_msg.eq(another_read_msg));
        assert_eq!(1, e.read_count());

        assert!(expected_msg.eq(read_msg));
    }
}
