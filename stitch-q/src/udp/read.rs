use async_std::io::*;
use async_std::net::*;
use async_std::task;
use crossbeam_channel::{Receiver, Sender, unbounded, bounded};

pub struct ReadOnlyUdpChannel<T>
where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub(crate) rx_chan: (Sender<T>, Receiver<T>),
    pub(crate) task:  task::JoinHandle<anyhow::Result<()>>
}

impl<T> ReadOnlyUdpChannel<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded<A: ToSocketAddrs>(ip_addrs: A, port: Option<usize>) -> Result<Self> {
        Self::from_parts(ip_addrs, port, unbounded())
    }

    pub fn bounded<A: ToSocketAddrs>(ip_addrs: A, port: Option<usize>, incoming_bound: Option<usize>) -> Result<Self> {
        let incoming_chan = if let Some(bound) = incoming_bound {
            bounded(bound)
        } else {
            unbounded()
        };

        Self::from_parts(ip_addrs, port, incoming_chan)
    }

    pub fn from_parts<A: ToSocketAddrs>(ip_addrs: A, port: Option<usize>, incoming_chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let socket = if let Some(port_num) = port {
            UdpSocket::bind(format!("127.0.0.1:{}", port_num))
        } else {
            UdpSocket::bind(String::from("127.0.0.1:0"))
        };

        Self::from_raw_parts(ip_addrs, task::block_on(socket)?, incoming_chan)
    }

    pub fn from_raw_parts<A: ToSocketAddrs>(ip_addrs: A, socket: UdpSocket, chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        task::block_on(socket.connect(ip_addrs))?;
        let sender = chan.0.clone();

        Ok(ReadOnlyUdpChannel {
            rx_chan: chan,
            task:    task::spawn(crate::dist_chan::udp::read_from_stream(socket, sender))
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
