use async_channel::{unbounded, Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use log::*;

use crate::net::registry::{StitchRegistry, StitchRegistryEntry, StitchRegistryKey};
use crate::net::StitchNetClientAgent;
use crate::{channel_factory, StitchMessage};
use async_std::sync::Arc;
use dashmap::mapref::one::Ref;
use std::any::Any;

pub fn tcp_client<A: ToSocketAddrs + std::fmt::Display>(
    ip_addrs: A,
) -> Result<StitchNetClientAgent> {
    tcp_client_with_bound(ip_addrs, None)
}

pub fn tcp_client_with_bound<A: ToSocketAddrs + std::fmt::Display>(
    ip_addrs: A,
    cap: Option<usize>,
) -> Result<StitchNetClientAgent> {
    let read_stream = task::block_on(TcpStream::connect(&ip_addrs))?;
    info!("Established client TCP connection to {}", ip_addrs);

    let write_stream = read_stream.clone();

    tcp_client_from_parts((read_stream, write_stream), channel_factory(cap))
}

pub fn tcp_client_from_parts(
    (read_stream, write_stream): (TcpStream, TcpStream),
    (tcp_write_sender, tcp_write_receiver): (Sender<StitchMessage>, Receiver<StitchMessage>),
) -> Result<StitchNetClientAgent> {
    let local_addr = read_stream.local_addr()?;
    let peer_addr = read_stream.peer_addr()?;

    let registry: StitchRegistry = crate::net::registry::new();

    let read_task = task::spawn(crate::net::tasks::read_from_stream(
        registry.clone(),
        read_stream,
    ));
    let write_task = task::spawn(crate::net::tasks::write_to_stream(
        tcp_write_receiver.clone(),
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
        read_task,
        write_task,
        stream_writer_chan: (tcp_write_sender, tcp_write_receiver),
    })
}
