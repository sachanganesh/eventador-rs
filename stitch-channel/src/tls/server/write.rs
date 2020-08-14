use async_channel::{Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use async_tls::{server::TlsStream, TlsAcceptor};
use futures_util::io::{AsyncReadExt, WriteHalf};

pub struct WriteOnlyTlsServerChannel<T>
where
    T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    pub(crate) tx_chan: (Sender<T>, Receiver<T>),
    pub(crate) task: task::JoinHandle<anyhow::Result<()>>,
}

impl<T> WriteOnlyTlsServerChannel<T>
where
    T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    pub fn unbounded(stream: TcpStream, acceptor: TlsAcceptor) -> Result<Self> {
        Self::from_parts(stream, acceptor, None)
    }

    pub fn bounded(
        stream: TcpStream,
        acceptor: TlsAcceptor,
        outgoing_bound: Option<usize>,
    ) -> Result<Self> {
        Self::from_parts(stream, acceptor, outgoing_bound)
    }

    pub fn from_parts(
        stream: TcpStream,
        acceptor: TlsAcceptor,
        outgoing_bound: Option<usize>,
    ) -> Result<Self> {
        stream.set_nodelay(true)?;
        let encrypted_stream = task::block_on(acceptor.accept(stream))?;
        let (_, write_stream) = encrypted_stream.split();

        Self::from_raw_parts(write_stream, crate::channel_factory(outgoing_bound))
    }

    pub(crate) fn from_raw_parts(
        stream: WriteHalf<TlsStream<TcpStream>>,
        chan: (Sender<T>, Receiver<T>),
    ) -> Result<Self> {
        let receiver = chan.1.clone();

        Ok(WriteOnlyTlsServerChannel {
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
