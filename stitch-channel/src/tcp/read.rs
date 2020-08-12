use async_std::io::*;
use async_std::net::*;
use async_std::task;
use async_channel::{Receiver, Sender, unbounded, bounded};

pub struct ReadOnlyTcpChannel<T>
where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub(crate) rx_chan: (Sender<T>, Receiver<T>),
    pub(crate) task:  task::JoinHandle<anyhow::Result<()>>
}

impl<T> ReadOnlyTcpChannel<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded<A: ToSocketAddrs>(ip_addrs: A) -> Result<Self> {
        Self::from_parts(ip_addrs, unbounded())
    }

    pub fn bounded<A: ToSocketAddrs>(ip_addrs: A, incoming_bound: Option<usize>) -> Result<Self> {
        let incoming_chan = if let Some(bound) = incoming_bound {
            bounded(bound)
        } else {
            unbounded()
        };

        Self::from_parts(ip_addrs, incoming_chan)
    }

    pub fn from_parts<A: ToSocketAddrs>(ip_addrs: A, incoming_chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let read_stream = task::block_on(TcpStream::connect(ip_addrs))?;

        Self::from_raw_parts(read_stream, incoming_chan)
    }

    pub(crate) fn from_raw_parts(stream: TcpStream, chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let sender = chan.0.clone();

        Ok(ReadOnlyTcpChannel {
            rx_chan: chan,
            task:    task::spawn(crate::tcp::read_from_stream(stream, sender))
        })
    }

    pub(crate) fn channel(&self) -> (Sender<T>, Receiver<T>) {
        (self.rx_chan.0.clone(), self.rx_chan.1.clone())
    }

    pub fn receiver(&self) -> Receiver<T> {
        self.rx_chan.1.clone()
    }

    pub fn task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.task
    }

    pub fn close(self) {
        task::block_on(self.task.cancel());
    }
}
