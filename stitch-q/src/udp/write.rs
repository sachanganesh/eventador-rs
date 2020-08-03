use async_std::io::*;
use async_std::net::*;
use async_std::task;
use crossbeam_channel::{Receiver, Sender, unbounded, bounded};

pub struct WriteOnlyUdpChannel<T>
where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub(crate) tx_chan: (Sender<T>, Receiver<T>),
    pub(crate) task:  task::JoinHandle<anyhow::Result<()>>
}

impl<T> WriteOnlyUdpChannel<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded<A: ToSocketAddrs>(ip_addrs: A, port: Option<usize>) -> Result<Self> {
        Self::from_parts(ip_addrs, port, unbounded())
    }

    pub fn bounded<A: ToSocketAddrs>(ip_addrs: A, port: Option<usize>, outgoing_bound: Option<usize>) -> Result<Self> {
        let outgoing_chan = if let Some(bound) = outgoing_bound {
            bounded(bound)
        } else {
            unbounded()
        };

        Self::from_parts(ip_addrs, port, outgoing_chan)
    }

    pub fn from_parts<A: ToSocketAddrs>(ip_addrs: A, port: Option<usize>, outgoing_chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let socket = if let Some(port_num) = port {
            UdpSocket::bind(format!("127.0.0.1:{}", port_num))
        } else {
            UdpSocket::bind(String::from("127.0.0.1:0"))
        };

        Self::from_raw_parts(ip_addrs, task::block_on(socket)?, outgoing_chan)
    }

    pub fn from_raw_parts<A: ToSocketAddrs>(ip_addrs: A, socket: UdpSocket, chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        task::block_on(socket.connect(ip_addrs))?;
        let receiver = chan.1.clone();

        Ok(WriteOnlyUdpChannel {
            tx_chan: chan,
            task:    task::spawn(crate::dist_chan::udp::write_to_stream(receiver, socket))
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
