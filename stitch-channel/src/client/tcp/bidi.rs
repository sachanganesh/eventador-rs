use async_std::io::*;
use async_std::net::*;
use async_std::task;
use crossbeam_channel::{Receiver, Sender, unbounded, bounded};

use crate::client::tcp::{read, write};

pub struct BiDirectionalTcpClient<T>
    where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    reader: read::ReadOnlyTcpClient<T>,
    writer: write::WriteOnlyTcpClient<T>
}

impl<T> BiDirectionalTcpClient<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded<A: ToSocketAddrs>(ip_addrs: A) -> Result<Self> {
        Self::from_parts(ip_addrs, unbounded(), unbounded())
    }

    pub fn bounded<A: ToSocketAddrs>(ip_addrs: A, outgoing_bound: Option<usize>, incoming_bound: Option<usize>) -> Result<Self> {
        let outgoing_chan = if let Some(bound) = outgoing_bound {
            bounded(bound)
        } else {
            unbounded()
        };

        let incoming_chan = if let Some(bound) = incoming_bound {
            bounded(bound)
        } else {
            unbounded()
        };

        Self::from_parts(ip_addrs, outgoing_chan, incoming_chan)
    }

    pub fn from_parts<A: ToSocketAddrs>(ip_addrs: A, outgoing_chan: (Sender<T>, Receiver<T>), incoming_chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let read_stream  = task::block_on(TcpStream::connect(ip_addrs))?;
        let write_stream = read_stream.clone();

        Ok(BiDirectionalTcpClient {
            reader: read::ReadOnlyTcpClient::from_raw_parts(read_stream, incoming_chan)?,
            writer: write::WriteOnlyTcpClient::from_raw_parts(write_stream, outgoing_chan)?
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
