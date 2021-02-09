use eventador::Eventador;

const NUM_EVENTS: usize = 1000;

fn main() -> anyhow::Result<()> {
    let eventbus = Eventador::new(2)?;

    let subscriber = eventbus.subscribe::<usize>();
    let subscriber_thread = std::thread::spawn(move || {
        let mut event_ctr = 0;
        while event_ctr < NUM_EVENTS {
            // std::thread::sleep(std::time::Duration::from_micros(10));

            let event = subscriber.recv();
            println!("Received event: {}", *event);

            assert_eq!(event_ctr + 1, *event);
            event_ctr += 1;
        }
    });

    let mut publisher = eventbus.publisher();
    let _publisher_thread = std::thread::spawn(move || {
        let mut i: usize = 1;
        while i <= NUM_EVENTS {
            // std::thread::sleep(std::time::Duration::from_micros(10));

            if i % 2 == 0 {
                publisher.send(i);
            } else {
                eventbus.publish(i);
            }
            println!("Published event: {}", i);

            i += 1;
        }
    });

    subscriber_thread
        .join()
        .expect("Join of subscriber thread was unsuccessful");

    Ok(())
}
