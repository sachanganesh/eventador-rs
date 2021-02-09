use eventador::{Eventador, SinkExt, StreamExt};

const NUM_EVENTS: usize = 1000;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let eventbus = Eventador::new(2)?;

    let subscriber_bus = eventbus.clone();
    let subscriber_task = async_std::task::spawn(async move {
        let mut subscriber = subscriber_bus.async_subscriber::<usize>();

        let mut event_ctr = 0;
        while event_ctr < NUM_EVENTS {
            // async_std::task::sleep(std::time::Duration::from_micros(10)).await;
            let event = subscriber
                .next()
                .await
                .expect("stream of subscribed events closed");
            println!("Received event: {}", *event);
            assert_eq!(event_ctr + 1, *event);
            event_ctr += 1;
        }
    });

    let publisher_task = async_std::task::spawn(async move {
        let mut publisher = eventbus.async_publisher::<usize>(4);
        let mut i: usize = 1;
        while i <= NUM_EVENTS {
            // async_std::task::sleep(std::time::Duration::from_micros(10)).await;
            publisher
                .send(i)
                .await
                .expect("publisher could not publish event");
            println!("Published event: {}", i);
            i += 1;
        }
    });

    publisher_task.await;
    subscriber_task.await;
    Ok(())
}
