use async_channel::{Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use log::*;

use crate::net::registry::StitchRegistry;
use crate::net::StitchNetClient;
use crate::{channel_factory, StitchMessage};
use async_std::sync::{Arc, Condvar, Mutex};

impl StitchNetClient {
    pub fn tcp_client<A: ToSocketAddrs + std::fmt::Display>(ip_addrs: A) -> Result<Self> {
        Self::tcp_client_with_bound(ip_addrs, None)
    }

    pub fn tcp_client_with_bound<A: ToSocketAddrs + std::fmt::Display>(
        ip_addrs: A,
        cap: Option<usize>,
    ) -> Result<Self> {
        let read_stream = task::block_on(TcpStream::connect(&ip_addrs))?;
        info!("Established client TCP connection to {}", ip_addrs);

        let write_stream = read_stream.clone();

        Self::tcp_client_from_parts((read_stream, write_stream), channel_factory(cap))
    }

    pub fn tcp_client_from_parts(
        (read_stream, write_stream): (TcpStream, TcpStream),
        (tcp_write_sender, tcp_write_receiver): (Sender<StitchMessage>, Receiver<StitchMessage>),
    ) -> Result<Self> {
        let local_addr = read_stream.local_addr()?;
        let peer_addr = read_stream.peer_addr()?;

        let registry: StitchRegistry = crate::net::registry::new();
        let read_readiness = Arc::new((Mutex::new(false), Condvar::new()));
        let write_readiness = Arc::new((Mutex::new(false), Condvar::new()));

        let read_task = task::spawn(crate::net::tasks::read_from_stream(
            registry.clone(),
            read_stream,
            read_readiness.clone(),
        ));

        let write_task = task::spawn(crate::net::tasks::write_to_stream(
            tcp_write_receiver.clone(),
            write_stream,
            write_readiness.clone(),
        ));

        Ok(Self {
            local_addr,
            peer_addr,
            registry,
            stream_writer_chan: (tcp_write_sender, tcp_write_receiver),
            read_readiness,
            write_readiness,
            read_task,
            write_task,
        })
    }
}
