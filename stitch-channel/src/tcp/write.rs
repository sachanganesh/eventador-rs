use async_channel::{Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use log::info;

pub struct WriteOnlyTcpChannel<T>
where
    T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    pub(crate) tx_chan: (Sender<T>, Receiver<T>),
    pub(crate) task: task::JoinHandle<anyhow::Result<()>>,
}

impl<T> WriteOnlyTcpChannel<T>
where
    T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    pub fn unbounded<A: ToSocketAddrs + std::fmt::Display>(ip_addrs: A) -> Result<Self> {
        Self::from_parts(ip_addrs, None)
    }

    pub fn bounded<A: ToSocketAddrs + std::fmt::Display>(ip_addrs: A, outgoing_bound: Option<usize>) -> Result<Self> {
        Self::from_parts(ip_addrs, outgoing_bound)
    }

    pub fn from_parts<A: ToSocketAddrs + std::fmt::Display>(
        ip_addrs: A,
        outgoing_bound: Option<usize>,
    ) -> Result<Self> {
        info!("Creating client TCP connection to {}", ip_addrs);
        let write_stream = task::block_on(TcpStream::connect(ip_addrs))?;
        write_stream.set_nodelay(true)?;

        Self::from_raw_parts(write_stream, crate::channel_factory(outgoing_bound))
    }

    pub fn from_raw_parts(stream: TcpStream, chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let receiver = chan.1.clone();

        Ok(WriteOnlyTcpChannel {
            tx_chan: chan,
            task: task::spawn(crate::write_to_stream(receiver, stream)),
        })
    }

    pub(crate) fn channel(&self) -> (Sender<T>, Receiver<T>) {
        (self.tx_chan.0.clone(), self.tx_chan.1.clone())
    }

    pub fn sender(&self) -> Sender<T> {
        self.tx_chan.0.clone()
    }

    pub fn task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.task
    }

    pub fn close(self) {
        task::block_on(self.task.cancel());
    }
}
