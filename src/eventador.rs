use crate::futures::publisher::AsyncPublisher;
use crate::futures::subscriber::AsyncSubscriber;
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crate::subscriber::Subscriber;
use std::sync::Arc;

#[derive(Clone)]
pub struct Eventador {
    ring: Arc<RingBuffer>,
}

impl Eventador {
    pub fn new(capacity: u64) -> anyhow::Result<Self> {
        Ok(Self {
            ring: Arc::new(RingBuffer::new(capacity)?),
        })
    }

    pub fn publish<T: 'static + Send>(&self, message: T) {
        let sequence = self.ring.next();

        if let Some(event_store) = self.ring.get_envelope(sequence).clone() {
            event_store.overwrite::<T>(sequence, message);
        }
    }

    pub fn subscribe<'a, T: 'static + Send>(&'a self) -> Subscriber<'a, T> {
        let sequence = Arc::new(Sequence::with_value(self.ring.sequencer().get() + 1));
        self.ring
            .sequencer()
            .register_gating_sequence(sequence.clone());

        Subscriber::new(self.ring.as_ref(), sequence)
    }

    pub fn async_publisher<T: 'static + Send + Unpin>(&self) -> AsyncPublisher<T> {
        AsyncPublisher::new(self.ring.clone())
    }

    pub fn async_subscriber<T: 'static + Send + Unpin>(&self) -> AsyncSubscriber<T> {
        let sequence = Arc::new(Sequence::with_value(self.ring.sequencer().get() + 1));
        self.ring
            .sequencer()
            .register_gating_sequence(sequence.clone());

        AsyncSubscriber::new(self.ring.clone(), sequence)
    }
}

impl From<RingBuffer> for Eventador {
    fn from(ring: RingBuffer) -> Self {
        Self {
            ring: Arc::new(ring),
        }
    }
}

impl From<Arc<RingBuffer>> for Eventador {
    fn from(ring: Arc<RingBuffer>) -> Self {
        Self { ring }
    }
}

#[cfg(test)]
mod tests {
    use crate::eventador::Eventador;
    use crate::futures::publisher::AsyncPublisher;
    use async_channel::{unbounded, RecvError};
    use futures::SinkExt;

    #[test]
    fn publish_and_subscribe() {
        let res = Eventador::new(4);
        assert!(res.is_ok());

        let disruptor: Eventador = res.unwrap();

        let subscriber = disruptor.subscribe::<usize>();
        assert_eq!(1, subscriber.sequence()); // @todo double check if it should be this way

        let mut i: usize = 1234;
        disruptor.publish(i);

        let mut msg = subscriber.recv().unwrap();
        assert_eq!(i, *msg);

        i += 1;
        let disruptor2 = disruptor.clone();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            disruptor2.publish(i);
        });

        msg = subscriber.recv().unwrap();
        assert_eq!(i, *msg);
    }

    #[async_std::test]
    async fn async_publish() {
        let res = Eventador::new(4);
        assert!(res.is_ok());

        let disruptor: Eventador = res.unwrap();

        let subscriber = disruptor.async_subscriber::<usize>();
        let mut publisher: AsyncPublisher<usize> = disruptor.async_publisher();

        let (sender, mut receiver) = unbounded::<Result<usize, RecvError>>();

        let mut i: usize = 1234;
        let mut sent = sender.send(Ok(i)).await;
        assert!(sent.is_ok());

        let _handle = async_std::task::spawn(async move {
            publisher.send_all(&mut receiver).await.unwrap();
        });

        let mut msg = subscriber.recv().await.unwrap();
        assert_eq!(i, *msg);

        i += 1;
        let disruptor2 = disruptor.clone();

        async_std::task::spawn(async move {
            async_std::task::sleep(std::time::Duration::from_secs(3)).await;
            disruptor2.publish(i);
        });

        msg = subscriber.recv().await.unwrap();
        assert_eq!(i, *msg);

        i += 1;
        sent = sender.send(Ok(i)).await;
        assert!(sent.is_ok());

        msg = subscriber.recv().await.unwrap();
        assert_eq!(i, *msg);
    }

    #[derive(Debug, Eq, PartialEq)]
    enum TestEnum {
        SampleA,
    }

    #[test]
    fn enum_specific_subscription() {
        let res = Eventador::new(4);
        assert!(res.is_ok());

        let disruptor: Eventador = res.unwrap();

        let subscriber = disruptor.subscribe::<TestEnum>();
        assert_eq!(1, subscriber.sequence()); // @todo double check if it should be this way

        disruptor.publish(TestEnum::SampleA);

        let msg = subscriber.recv().unwrap();
        assert_eq!(TestEnum::SampleA, *msg);
    }
}
