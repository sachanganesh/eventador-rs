use crate::channel_factory;
use crate::net::registry::StitchRegistry;
use crate::net::{ServerRegistry, StitchClient, StitchNetClient, StitchNetServer};
use async_channel::{Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::prelude::*;
use async_std::sync::{Arc, Condvar, Mutex};
use async_std::task;
use async_tls::TlsAcceptor;
use dashmap::DashMap;
use futures_util::AsyncReadExt;
use log::*;

impl StitchNetServer {
    pub fn tls_server<A: ToSocketAddrs + std::fmt::Display>(
        ip_addrs: A,
        acceptor: TlsAcceptor,
    ) -> Result<(StitchNetServer, Receiver<Arc<StitchNetClient>>)> {
        Self::tls_server_with_bound(ip_addrs, acceptor, None)
    }

    pub fn tls_server_with_bound<A: ToSocketAddrs + std::fmt::Display>(
        ip_addrs: A,
        acceptor: TlsAcceptor,
        cap: Option<usize>,
    ) -> Result<(Self, Receiver<Arc<StitchNetClient>>)> {
        let listener = task::block_on(TcpListener::bind(ip_addrs))?;
        info!("Started TLS server at {}", listener.local_addr()?);

        let registry = Arc::new(DashMap::new());
        let (sender, receiver) = channel_factory(cap);

        let handler = task::spawn(handle_server_connections(
            acceptor,
            registry.clone(),
            listener,
            sender.clone(),
            cap,
        ));

        Ok((
            Self {
                registry,
                connections_chan: (sender, receiver.clone()),
                accept_loop_task: handler,
            },
            receiver,
        ))
    }
}

async fn handle_server_connections<'a>(
    acceptor: TlsAcceptor,
    registry: ServerRegistry,
    input: TcpListener,
    output: Sender<Arc<StitchNetClient>>,
    cap: Option<usize>,
) -> anyhow::Result<()> {
    let mut conns = input.incoming();

    debug!("Reading from the stream of incoming connections");
    loop {
        match conns.next().await {
            Some(Ok(tcp_stream)) => {
                let local_addr = tcp_stream.local_addr()?;
                let peer_addr = tcp_stream.peer_addr()?;

                debug!("Received connection attempt from {}", peer_addr);

                let tls_stream = acceptor.accept(tcp_stream).await?;

                let (read_stream, write_stream) = tls_stream.split();
                let (tls_write_sender, tls_write_receiver) = channel_factory(cap);

                let client_registry: StitchRegistry = crate::net::registry::new();
                let read_readiness = Arc::new((Mutex::new(false), Condvar::new()));
                let write_readiness = Arc::new((Mutex::new(false), Condvar::new()));

                let read_task = task::spawn(crate::net::tasks::read_from_stream(
                    client_registry.clone(),
                    read_stream,
                    read_readiness.clone(),
                ));

                let write_task = task::spawn(crate::net::tasks::write_to_stream(
                    tls_write_receiver.clone(),
                    write_stream,
                    write_readiness.clone(),
                ));

                let conn = StitchNetClient {
                    local_addr,
                    peer_addr,
                    registry: client_registry,
                    stream_writer_chan: (tls_write_sender, tls_write_receiver),
                    read_readiness,
                    write_readiness,
                    read_task,
                    write_task,
                };

                debug!("Attempting to register connection from {}", peer_addr);
                let conn = Arc::new(conn);
                registry.insert(conn.peer_addr(), conn.clone());
                debug!("Registered client connection for {} in server registry", peer_addr);

                if let Err(err) = output.send(conn).await {
                    error!(
                        "Stopping the server accept loop - could not send accepted TLS client connection to channel: {:#?}",
                        err
                    );

                    break Err(anyhow::Error::from(err))
                } else {
                    info!("Accepted connection from {}", peer_addr);
                }
            }

            Some(Err(err)) => error!(
                "Encountered error when accepting TLS connection: {:#?}",
                err
            ),

            None => {
                warn!(
                    "Stopping the server accept loop - unable to accept any more connections"
                );

                break Ok(())
            },
        }
    }
}
