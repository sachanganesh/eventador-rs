use async_std::io::*;
use async_std::net::*;
use async_std::task;
use async_tls::{TlsConnector, client::TlsStream};
use async_channel::{Receiver, Sender};
use futures_util::io::{AsyncReadExt, ReadHalf};

pub struct ReadOnlyTlsChannel<T>
where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub(crate) rx_chan: (Sender<T>, Receiver<T>),
    pub(crate) task:  task::JoinHandle<anyhow::Result<()>>
}

impl<T> ReadOnlyTlsChannel<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded<A: ToSocketAddrs + std::convert::AsRef<str>>(ip_addrs: A) -> Result<Self> {
        Self::from_parts(ip_addrs, None)
    }

    pub fn bounded<A: ToSocketAddrs + std::convert::AsRef<str>>(ip_addrs: A, incoming_bound: Option<usize>) -> Result<Self> {
        Self::from_parts(ip_addrs, incoming_bound)
    }

    pub fn from_parts<A: ToSocketAddrs + std::convert::AsRef<str>>(ip_addrs: A, incoming_bound: Option<usize>) -> Result<Self> {
        let socket_addrs = task::block_on(ip_addrs.to_socket_addrs())?.next().unwrap();
        let read_stream = task::block_on(TcpStream::connect(&socket_addrs))?;

        let connector = TlsConnector::default();
        let encrypted_stream = task::block_on(connector.connect(ip_addrs, read_stream))?;
        let (read_stream, _) = encrypted_stream.split();

        Self::from_raw_parts(read_stream, crate::channel_factory(incoming_bound))
    }

    pub fn from_raw_parts(stream: ReadHalf<TlsStream<TcpStream>>, chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let sender = chan.0.clone();

        Ok(ReadOnlyTlsChannel {
            rx_chan: chan,
            task:    task::spawn(crate::read_from_stream(stream, sender))
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
