use std::time::Duration;

#[derive(Copy, Clone)]
pub enum WaitStrategy {
    AllSubscribers,
    NoWait,
    WaitForDuration(Duration),
}

#[cfg(test)]
mod tests {
    use crate::{Eventador, WaitStrategy};

    #[test]
    fn test_wait_for_all_subscribers() {
        let eventbus = Eventador::new(2).unwrap();

        let subscriber = eventbus.subscribe::<usize>();

        let _publish_thread = std::thread::spawn(move || {
            for i in 0..3 {
                let i: usize = i;
                eventbus.publish(i)
            }
        });

        std::thread::sleep(std::time::Duration::from_secs(1));
        let i: usize = 0;
        let msg = subscriber.recv();
        assert_eq!(i, *msg);
    }

    #[test]
    fn test_no_wait() {
        let eventbus = Eventador::with_strategy(2, WaitStrategy::NoWait).unwrap();

        let subscriber = eventbus.subscribe::<usize>();

        let _publish_thread = std::thread::spawn(move || {
            for i in 0..3 {
                let i: usize = i;
                eventbus.publish(i);
            }
        });

        std::thread::sleep(std::time::Duration::from_secs(1));
        let i: usize = 2;
        let msg = subscriber.recv();
        assert_eq!(i, *msg);
    }

    #[test]
    fn test_wait_for_duration() {
        let eventbus = Eventador::with_strategy(
            2,
            WaitStrategy::WaitForDuration(std::time::Duration::from_secs(1)),
        )
        .unwrap();

        let subscriber1 = eventbus.subscribe::<usize>();
        let subscriber2 = eventbus.subscribe::<usize>();

        let _publish_thread = std::thread::spawn(move || {
            for i in 0..3 {
                let i: usize = i;
                eventbus.publish(i);
            }
        });

        let i: usize = 0;
        let msg = subscriber1.recv();
        assert_eq!(i, *msg);

        std::thread::sleep(std::time::Duration::from_secs(3));
        let i: usize = 2;
        let msg = subscriber2.recv();
        assert_eq!(i, *msg);
    }
}
