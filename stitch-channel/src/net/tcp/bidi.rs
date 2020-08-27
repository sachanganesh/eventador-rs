use async_channel::{unbounded, Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use log::*;

// use crate::net::{read, write};
use crate::net::registry::{StitchRegistry, StitchRegistryEntry, StitchRegistryKey};
use crate::{channel_factory, StitchMessage};
use dashmap::mapref::one::Ref;
use std::any::Any;
use std::sync::Arc;

pub struct BidirectionalTcpAgent {
    registry: StitchRegistry,
    read_task: task::JoinHandle<anyhow::Result<()>>,
    write_task: task::JoinHandle<anyhow::Result<()>>,
    stream_writer_chan: (Sender<StitchMessage>, Receiver<StitchMessage>),
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
        let registry: StitchRegistry = crate::net::registry::new_stitch_registry();

        let read_task = task::spawn(crate::net::read_from_stream(registry.clone(), read_stream));
        let write_task = task::spawn(crate::net::write_to_stream(receiver.clone(), write_stream));

        Ok(Self {
            registry,
            read_task,
            write_task,
            stream_writer_chan: (sender, receiver),
        })
    }

    pub fn stream_writer_chan(&self) -> (Sender<StitchMessage>, Receiver<StitchMessage>) {
        (
            self.stream_writer_chan.0.clone(),
            self.stream_writer_chan.1.clone(),
        )
    }

    pub fn read_task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.read_task
    }

    pub fn write_task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.write_task
    }

    pub fn close(self) {
        self.stream_writer_chan.0.close();
        self.stream_writer_chan.1.close();
    }

    fn get_key_from_type<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &self,
    ) -> StitchRegistryKey {
        StitchMessage::hash_type::<T>()
    }

    pub fn channel_exists<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &self,
    ) -> bool {
        let key = self.get_key_from_type::<T>();
        self.registry.contains_key(&key)
    }

    pub fn get_channel<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &self,
    ) -> anyhow::Result<(Sender<T>, Receiver<T>)> {
        let key = self.get_key_from_type::<T>();

        match self.registry.get(&key) {
            Some(entry) => entry.user_facing_chan(),

            None => {
                info!("Could not find entry for type-id {} in the registry", key);

                Err(anyhow::Error::msg(format!(
                    "channel of type-id {} not registered",
                    key
                )))
            }
        }
    }

    pub fn unbounded<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &mut self,
    ) -> (Sender<T>, Receiver<T>) {
        self.bounded(None)
    }

    pub fn bounded<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &mut self,
        cap: Option<usize>,
    ) -> (Sender<T>, Receiver<T>) {
        if let Ok(chan) = self.get_channel::<T>() {
            return chan;
        }

        let tid_hash: StitchRegistryKey = StitchMessage::hash_type::<T>();

        let (serializer_sender, serializer_receiver): (Sender<T>, Receiver<T>) =
            channel_factory(cap);
        let (stream_writer_sender, _) = self.stream_writer_chan();

        let (deserializer_sender, deserializer_receiver) = unbounded::<StitchMessage>();
        let (user_sender, user_receiver): (Sender<T>, Receiver<T>) = channel_factory(cap);

        self.registry.insert(
            tid_hash,
            Arc::new(StitchRegistryEntry::new(
                (serializer_sender.clone(), serializer_receiver.clone()),
                (deserializer_sender.clone(), deserializer_receiver.clone()),
                (user_sender.clone(), user_receiver.clone()),
                task::spawn(crate::net::serialize::<T>(
                    serializer_receiver,
                    stream_writer_sender,
                )),
                task::spawn(crate::net::deserialize::<T>(
                    deserializer_receiver,
                    user_sender,
                )),
            )),
        );

        return (serializer_sender, user_receiver);
    }
}
