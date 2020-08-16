use async_channel::{Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use log::info;

use crate::tcp::{read, write};

pub struct BiDirectionalTcpChannel<T>
where
    T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    reader: read::ReadOnlyTcpChannel<T>,
    writer: write::WriteOnlyTcpChannel<T>,
}

impl<T> BiDirectionalTcpChannel<T>
where
    T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    pub fn unbounded<A: ToSocketAddrs + std::fmt::Display>(ip_addrs: A) -> Result<Self> {
        Self::from_parts(ip_addrs, None, None)
    }

    pub fn bounded<A: ToSocketAddrs + std::fmt::Display>(
        ip_addrs: A,
        outgoing_bound: Option<usize>,
        incoming_bound: Option<usize>,
    ) -> Result<Self> {
        Self::from_parts(ip_addrs, outgoing_bound, incoming_bound)
    }

    pub fn from_parts<A: ToSocketAddrs + std::fmt::Display>(
        ip_addrs: A,
        outgoing_bound: Option<usize>,
        incoming_bound: Option<usize>,
    ) -> Result<Self> {
        info!("Creating client TCP connection to {}", ip_addrs);
        let read_stream = task::block_on(TcpStream::connect(ip_addrs))?;
        let write_stream = read_stream.clone();

        Self::from_raw_parts(
            (read_stream, write_stream),
            crate::channel_factory(outgoing_bound),
            crate::channel_factory(incoming_bound),
        )
    }

    pub(crate) fn from_raw_parts(
        (read_stream, write_stream): (TcpStream, TcpStream),
        outgoing_chan: (Sender<T>, Receiver<T>),
        incoming_chan: (Sender<T>, Receiver<T>),
    ) -> Result<Self> {
        Ok(BiDirectionalTcpChannel {
            reader: read::ReadOnlyTcpChannel::from_raw_parts(read_stream, incoming_chan)?,
            writer: write::WriteOnlyTcpChannel::from_raw_parts(write_stream, outgoing_chan)?,
        })
    }

    pub fn channel(&self) -> (Sender<T>, Receiver<T>) {
        (self.writer.tx_chan.0.clone(), self.reader.rx_chan.1.clone())
    }

    fn tcp_channel(&self) -> (&Sender<T>, &Receiver<T>) {
        (&self.reader.rx_chan.0, &self.writer.tx_chan.1)
    }

    pub fn read_task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.reader.task
    }

    pub fn write_task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.writer.task
    }

    pub fn close(self) {
        self.reader.close();
        self.writer.close();
    }
}
