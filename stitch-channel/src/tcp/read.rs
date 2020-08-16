use async_channel::{Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use log::info;

pub struct ReadOnlyTcpChannel<T>
where
    T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    pub(crate) rx_chan: (Sender<T>, Receiver<T>),
    pub(crate) task: task::JoinHandle<anyhow::Result<()>>,
}

impl<T> ReadOnlyTcpChannel<T>
where
    T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    pub fn unbounded<A: ToSocketAddrs + std::fmt::Display>(ip_addrs: A) -> Result<Self> {
        Self::from_parts(ip_addrs, None)
    }

    pub fn bounded<A: ToSocketAddrs + std::fmt::Display>(ip_addrs: A, incoming_bound: Option<usize>) -> Result<Self> {
        Self::from_parts(ip_addrs, incoming_bound)
    }

    pub fn from_parts<A: ToSocketAddrs + std::fmt::Display>(
        ip_addrs: A,
        incoming_bound: Option<usize>,
    ) -> Result<Self> {
        info!("Creating client TCP connection to {}", ip_addrs);
        let read_stream = task::block_on(TcpStream::connect(ip_addrs))?;

        Self::from_raw_parts(read_stream, crate::channel_factory(incoming_bound))
    }

    pub(crate) fn from_raw_parts(
        stream: TcpStream,
        chan: (Sender<T>, Receiver<T>),
    ) -> Result<Self> {
        let sender = chan.0.clone();

        Ok(ReadOnlyTcpChannel {
            rx_chan: chan,
            task: task::spawn(crate::read_from_stream(stream, sender)),
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
