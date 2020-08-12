use async_std::io::*;
use async_std::net::*;
use async_std::task;
use async_tls::{TlsConnector, client::TlsStream};
use async_channel::{Receiver, Sender, unbounded, bounded};
use futures_util::io::{AsyncReadExt, WriteHalf};

pub struct WriteOnlyTlsClient<T>
where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub(crate) tx_chan: (Sender<T>, Receiver<T>),
    pub(crate) task:  task::JoinHandle<anyhow::Result<()>>
}

impl<T> WriteOnlyTlsClient<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded<A: ToSocketAddrs + std::convert::AsRef<str>>(ip_addrs: A) -> Result<Self> {
        Self::from_parts(ip_addrs, unbounded())
    }

    pub fn bounded<A: ToSocketAddrs + std::convert::AsRef<str>>(ip_addrs: A, outgoing_bound: Option<usize>) -> Result<Self> {
        let outgoing_chan = if let Some(bound) = outgoing_bound {
            bounded(bound)
        } else {
            unbounded()
        };

        Self::from_parts(ip_addrs, outgoing_chan)
    }

    pub fn from_parts<A: ToSocketAddrs + std::convert::AsRef<str>>(ip_addrs: A, outgoing_chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let socket_addrs = task::block_on(ip_addrs.to_socket_addrs())?.next().unwrap();
        let write_stream = task::block_on(TcpStream::connect(&socket_addrs))?;
        write_stream.set_nodelay(true)?;

        let connector = TlsConnector::default();
        let encrypted_stream = task::block_on(connector.connect(ip_addrs, write_stream))?;
        let (_, write_stream) = encrypted_stream.split();

        Self::from_raw_parts(write_stream, outgoing_chan)
    }

    pub(crate) fn from_raw_parts(stream: WriteHalf<TlsStream<TcpStream>>, chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let receiver = chan.1.clone();

        Ok(WriteOnlyTlsClient {
            tx_chan: chan,
            task:    task::spawn(crate::tls::write_to_stream(receiver, stream))
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
