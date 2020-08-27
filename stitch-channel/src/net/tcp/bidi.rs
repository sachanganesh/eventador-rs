use async_channel::{unbounded, Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use log::*;

// use crate::net::{read, write};
use crate::net::registry::{StitchRegistry, StitchRegistryEntry, StitchRegistryKey};
use crate::{channel_factory, StitchMessage};
use async_std::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub struct BidirectionalTcpAgent {
    registry: StitchRegistry,
    read_task: task::JoinHandle<anyhow::Result<()>>,
    write_task: task::JoinHandle<anyhow::Result<()>>,
    stitch_chan: (Sender<StitchMessage>, Receiver<StitchMessage>),
}

impl BidirectionalTcpAgent {
    pub fn new<A: ToSocketAddrs + std::fmt::Display>(ip_addrs: A) -> Result<Self> {
        Self::with_bound(ip_addrs, None)
    }

    pub fn with_bound<A: ToSocketAddrs + std::fmt::Display>(
        ip_addrs: A,
        cap: Option<usize>,
    ) -> Result<Self> {
        info!("Creating client TCP connection to {}", ip_addrs);
        let read_stream = task::block_on(TcpStream::connect(ip_addrs))?;
        let write_stream = read_stream.clone();

        let (sender, receiver) = channel_factory(cap);
        let registry: StitchRegistry = Arc::new(RwLock::new(HashMap::new()));

        let read_task = task::spawn(crate::net::read_from_stream(registry.clone(), read_stream));
        let write_task = task::spawn(crate::net::write_to_stream(receiver.clone(), write_stream));

        Ok(Self {
            registry,
            read_task,
            write_task,
            stitch_chan: (sender, receiver),
        })
    }

    pub fn stitch_chan(&self) -> (Sender<StitchMessage>, Receiver<StitchMessage>) {
        (self.stitch_chan.0.clone(), self.stitch_chan.1.clone())
    }

    pub fn read_task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.read_task
    }

    pub fn write_task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.write_task
    }

    pub fn close(self) {
        self.stitch_chan.0.close();
        self.stitch_chan.1.close();
    }

    pub fn unbounded<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &mut self,
    ) -> (Sender<Box<T>>, Receiver<Box<T>>) {
        let (stitch_sender, stitch_receiver) = self.stitch_chan();

        let tid_hash: StitchRegistryKey = StitchMessage::hash_type::<T>();

        let (se_sender, se_receiver) = unbounded();
        let (de_sender, de_receiver) = unbounded();

        let mut writable_registry = task::block_on(self.registry.write());
        debug!("Acquired write lock for registry");

        writable_registry.insert(
            tid_hash,
            Arc::new(StitchRegistryEntry::new(
                (se_sender.clone(), se_receiver.clone()),
                (de_sender.clone(), de_receiver.clone()),
                task::spawn(crate::net::serialize(se_receiver, stitch_sender.clone())),
                task::spawn(crate::net::deserialize(stitch_receiver.clone(), de_sender)),
            )),
        );

        debug!("Releasing write lock for registry");
        return (se_sender, de_receiver);
    }
}
