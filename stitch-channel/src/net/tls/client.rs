pub use async_tls;
pub use rustls;

use async_channel::{Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use async_std::task::JoinHandle;
use async_tls::{client::TlsStream, TlsConnector};
use futures_util::io::{AsyncReadExt, ReadHalf, WriteHalf};
use log::*;

use crate::net::registry::StitchRegistry;
use crate::net::{StitchNetClient, StitchNetClientAgent};
use crate::{channel_factory, StitchMessage};

pub fn tls_client<A: ToSocketAddrs + std::fmt::Display>(
    ip_addrs: A,
    domain: &str,
    connector: TlsConnector,
) -> Result<StitchNetClientAgent> {
    tls_client_with_bound(ip_addrs, domain, connector, None)
}

pub fn tls_client_with_bound<A: ToSocketAddrs + std::fmt::Display>(
    ip_addrs: A,
    domain: &str,
    connector: TlsConnector,
    cap: Option<usize>,
) -> Result<StitchNetClientAgent> {
    let stream = task::block_on(TcpStream::connect(&ip_addrs))?;
    stream.set_nodelay(true)?;
    info!("Established client TCP connection to {}", ip_addrs);

    tls_client_from_parts(stream, domain, connector, channel_factory(cap))
}

pub fn tls_client_from_parts(
    stream: TcpStream,
    domain: &str,
    connector: TlsConnector,
    (tls_write_sender, tls_write_receiver): (Sender<StitchMessage>, Receiver<StitchMessage>),
) -> Result<StitchNetClientAgent> {
    let local_addr = stream.local_addr()?;
    let peer_addr = stream.peer_addr()?;

    let encrypted_stream = task::block_on(connector.connect(domain, stream))?;
    let (read_stream, write_stream) = encrypted_stream.split();
    info!("Established client TLS connection to {}", peer_addr);

    let registry: StitchRegistry = crate::net::registry::new();

    let read_task = task::spawn(crate::net::tasks::read_from_stream(
        registry.clone(),
        read_stream,
    ));
    let write_task = task::spawn(crate::net::tasks::write_to_stream(
        tls_write_receiver.clone(),
        write_stream,
    ));
    info!(
        "Running serialize ({}) and deserialize ({}) tasks",
        read_task.task().id(),
        write_task.task().id()
    );

    Ok(StitchNetClientAgent {
        local_addr,
        peer_addr,
        registry,
        stream_writer_chan: (tls_write_sender, tls_write_receiver),
        read_task,
        write_task,
    })
}
