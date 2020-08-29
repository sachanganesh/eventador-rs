pub(crate) mod bidi;
pub(crate) mod read;
pub(crate) mod write;

use async_channel::{Receiver, Sender};
use async_std::net::*;
use async_std::prelude::StreamExt;
use async_std::task;
use async_tls::TlsAcceptor;
use futures_util::AsyncReadExt;
use log::*;

use crate::tls::BiDirectionalTlsServerChannel;

pub struct TlsServer {
    task: task::JoinHandle<Result<(), anyhow::Error>>,
}

impl TlsServer {
    pub fn unbounded<
        A: ToSocketAddrs,
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
        F: 'static + Send + async_std::future::Future,
        H: 'static + Send + FnMut((Sender<T>, Receiver<T>)) -> F,
    >(
        ip_addrs: A,
        acceptor: TlsAcceptor,
        mut connection_handler: H,
    ) -> Result<Self, anyhow::Error>
    where
        <F as std::future::Future>::Output: std::marker::Send,
    {
        let listener = task::block_on(TcpListener::bind(ip_addrs))?;

        Self::from_parts(listener, acceptor, connection_handler, None, None)
    }

    pub fn bounded<
        A: ToSocketAddrs,
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
        F: 'static + Send + async_std::future::Future,
        H: 'static + Send + FnMut((Sender<T>, Receiver<T>)) -> F,
    >(
        ip_addrs: A,
        acceptor: TlsAcceptor,
        mut connection_handler: H,
        outgoing_bound: Option<usize>,
        incoming_bound: Option<usize>,
    ) -> Result<Self, anyhow::Error>
    where
        <F as std::future::Future>::Output: std::marker::Send,
    {
        let listener = task::block_on(TcpListener::bind(ip_addrs))?;

        Self::from_parts(
            listener,
            acceptor,
            connection_handler,
            outgoing_bound,
            incoming_bound,
        )
    }

    pub fn from_parts<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
        F: 'static + Send + async_std::future::Future,
        H: 'static + Send + FnMut((Sender<T>, Receiver<T>)) -> F,
    >(
        listener: TcpListener,
        acceptor: TlsAcceptor,
        mut connection_handler: H,
        outgoing_bound: Option<usize>,
        incoming_bound: Option<usize>,
    ) -> Result<Self, anyhow::Error>
    where
        <F as std::future::Future>::Output: std::marker::Send,
    {
        let handler = task::spawn(async move {
            let mut incoming_stream = listener.incoming();

            // @todo implement recovery/retry mechanism
            loop {
                trace!("Reading from the stream of incoming connections");
                match incoming_stream.next().await {
                    Some(Ok(tcp_stream)) => {
                        let addr = tcp_stream.peer_addr()?;
                        let acceptor: TlsAcceptor = acceptor.clone();

                        let tls_stream = acceptor.accept(tcp_stream).await?;

                        let outgoing_chan = crate::channel_factory(outgoing_bound);
                        let incoming_chan = crate::channel_factory(incoming_bound);
                        match BiDirectionalTlsServerChannel::from_raw_parts(
                            tls_stream.split(),
                            outgoing_chan,
                            incoming_chan,
                        ) {
                            Ok(dist_chan) => {
                                info!(
                                    "Accepted a connection from {} and passing to handler fn",
                                    addr
                                );
                                task::spawn(connection_handler(dist_chan.channel()));
                            }

                            Err(err) => {
                                error!("Encountered error when creating TCP channel: {:#?}", err)
                            }
                        }
                    }

                    Some(Err(err)) => error!(
                        "Encountered error when accepting TCP connection: {:#?}",
                        err
                    ),
                    None => unreachable!(),
                }
            }
        });

        Ok(TlsServer { task: handler })
    }

    pub fn task(&self) -> &task::JoinHandle<Result<(), anyhow::Error>> {
        &self.task
    }

    pub fn close(self) {
        task::block_on(self.task.cancel());
    }
}
