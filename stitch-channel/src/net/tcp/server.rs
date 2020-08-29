use crate::channel_factory;
use crate::net::{ServerRegistry, StitchClient, StitchNetClient, StitchNetServer};
use async_channel::{bounded, unbounded, Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::prelude::*;
use async_std::sync::{Arc, RwLock};
use async_std::task;
use dashmap::DashMap;
use log::*;

impl StitchNetServer {
    pub fn tcp_server<A: ToSocketAddrs + std::fmt::Display>(
        ip_addrs: A,
    ) -> Result<(StitchNetServer, Receiver<Arc<StitchNetClient>>)> {
        Self::tcp_server_with_bound(ip_addrs, None)
    }

    pub fn tcp_server_with_bound<A: ToSocketAddrs + std::fmt::Display>(
        ip_addrs: A,
        cap: Option<usize>,
    ) -> Result<(Self, Receiver<Arc<StitchNetClient>>)> {
        let listener = task::block_on(TcpListener::bind(ip_addrs))?;
        info!("Started TCP server at {}", listener.local_addr()?);

        let registry = Arc::new(DashMap::new());
        let (sender, receiver) = channel_factory(cap);

        let handler = task::spawn(handle_server_connections(
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
    registry: ServerRegistry,
    input: TcpListener,
    output: Sender<Arc<StitchNetClient>>,
    cap: Option<usize>,
) -> anyhow::Result<()> {
    let mut conns = input.incoming();

    loop {
        debug!("Reading from the stream of incoming connections");
        match conns.next().await {
            Some(Ok(read_stream)) => {
                debug!(
                    "Received connection attempt from {}",
                    read_stream.peer_addr()?
                );
                let addr = read_stream.peer_addr()?;
                let write_stream = read_stream.clone();

                match StitchNetClient::tcp_client_from_parts(
                    (read_stream, write_stream),
                    channel_factory(cap),
                ) {
                    Ok(client) => {
                        info!("Accepted a connection from {}", addr);

                        let client = Arc::new(client);
                        registry.insert(client.peer_addr(), client.clone());

                        if let Err(err) = output.send(client).await {
                            warn!(
                                "Could not send accepted TCP client connection to channel: {:#?}",
                                err
                            )
                        }
                    }

                    Err(err) => error!(
                        "Encountered error when creating TCP client agent: {:#?}",
                        err
                    ),
                }
            }

            Some(Err(err)) => error!(
                "Encountered error when accepting TCP connection: {:#?}",
                err
            ),
            None => unreachable!(),
        }
    }
}
