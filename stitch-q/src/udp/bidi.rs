use async_std::io::*;
use async_std::net::*;
use async_std::task;
use crossbeam_channel::{Receiver, Sender, unbounded, bounded};

use crate::dist_chan::udp::{read, write};

pub struct BiDirectionalUdpChannel<T>
    where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    reader: read::ReadOnlyUdpChannel<T>,
    writer: write::WriteOnlyUdpChannel<T>
}

impl<T> BiDirectionalUdpChannel<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded(ip_addrs: std::net::SocketAddr, port: Option<usize>) -> Result<Self> {
        Self::from_parts(ip_addrs, port, unbounded(), unbounded())
    }

    pub fn bounded(ip_addrs: std::net::SocketAddr, port: Option<usize>, outgoing_bound: Option<usize>, incoming_bound: Option<usize>) -> Result<Self> {
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

        Self::from_parts(ip_addrs, port, outgoing_chan, incoming_chan)
    }

    pub fn from_parts(ip_addrs: std::net::SocketAddr, port: Option<usize>, outgoing_chan: (Sender<T>, Receiver<T>), incoming_chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let read_socket_raw = task::block_on(async move {
            if let Some(port_num) = port {
                std::net::UdpSocket::bind(format!("127.0.0.1:{}", port_num))
            } else {
                std::net::UdpSocket::bind(String::from("127.0.0.1:0"))
            }
        })?;
        let write_socket_raw = read_socket_raw.try_clone()?;

        println!("bound to socket port {}", read_socket_raw.local_addr()?);

        let read_socket  = UdpSocket::from(read_socket_raw);
        let write_socket = UdpSocket::from(write_socket_raw);

        Ok(BiDirectionalUdpChannel {
            reader: read::ReadOnlyUdpChannel::from_raw_parts(ip_addrs.clone(), read_socket, incoming_chan)?,
            writer: write::WriteOnlyUdpChannel::from_raw_parts(ip_addrs, write_socket, outgoing_chan)?
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
