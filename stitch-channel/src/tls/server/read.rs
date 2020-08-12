use async_std::io::*;
use async_std::net::*;
use async_std::task;
use async_tls::{TlsAcceptor, server::TlsStream};
use async_channel::{Receiver, Sender, unbounded, bounded};
use futures_util::io::{AsyncReadExt, ReadHalf};

pub struct ReadOnlyTlsServerChannel<T>
where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub(crate) rx_chan: (Sender<T>, Receiver<T>),
    pub(crate) task:  task::JoinHandle<anyhow::Result<()>>
}

impl<T> ReadOnlyTlsServerChannel<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded(stream: TcpStream, acceptor: TlsAcceptor) -> Result<Self> {
        Self::from_parts(stream, acceptor, unbounded())
    }

    pub fn bounded(stream: TcpStream, acceptor: TlsAcceptor, incoming_bound: Option<usize>) -> Result<Self> {
        let incoming_chan = if let Some(bound) = incoming_bound {
            bounded(bound)
        } else {
            unbounded()
        };

        Self::from_parts(stream, acceptor, incoming_chan)
    }

    pub fn from_parts(stream: TcpStream, acceptor: TlsAcceptor, incoming_chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let encrypted_stream = task::block_on( acceptor.accept(stream))?;
        let (read_stream, _) = encrypted_stream.split();

        Self::from_raw_parts(read_stream, incoming_chan)
    }

    pub fn from_raw_parts(stream: ReadHalf<TlsStream<TcpStream>>, chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let sender = chan.0.clone();

        Ok(ReadOnlyTlsServerChannel {
            rx_chan: chan,
            task:    task::spawn(crate::tls::server::read_from_stream(stream, sender))
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
