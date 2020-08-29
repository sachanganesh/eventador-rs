mod registry;
pub mod tasks;
pub mod tcp;
pub mod tls;

use crate::net::registry::{StitchRegistry, StitchRegistryEntry, StitchRegistryKey};
use crate::{channel_factory, StitchMessage};
use async_channel::{unbounded, Receiver, Sender};
use async_std::net::SocketAddr;
use async_std::prelude::*;
use async_std::sync::Arc;
use async_std::task;
use bytes::{Buf, BytesMut};
use futures_util::io::{AsyncRead, AsyncWrite};
use log::*;
use rmp_serde::{decode, encode};
use serde::Deserialize;
use std::io::Cursor;

pub trait StitchNetClient {
    fn registry(&self) -> StitchRegistry;
    fn stream_writer_chan(&self) -> (Sender<StitchMessage>, Receiver<StitchMessage>);

    fn local_addr(&self) -> SocketAddr;
    fn peer_addr(&self) -> SocketAddr;

    fn close(self);

    fn get_key_from_type<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &self,
    ) -> StitchRegistryKey {
        StitchMessage::hash_type::<T>()
    }

    fn channel_exists<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &self,
    ) -> bool {
        let key = self.get_key_from_type::<T>();
        self.registry().contains_key(&key)
    }

    fn get_channel<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &self,
    ) -> anyhow::Result<(Sender<T>, Receiver<T>)> {
        let key = self.get_key_from_type::<T>();

        match self.registry().get(&key) {
            Some(entry) => entry.user_facing_chan(),

            None => {
                debug!("Could not find entry for type-id {} in the registry", key);

                Err(anyhow::Error::msg(format!(
                    "channel of type-id {} not registered",
                    key
                )))
            }
        }
    }

    fn unbounded<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &self,
    ) -> (Sender<T>, Receiver<T>) {
        self.bounded(None)
    }

    fn bounded<
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
    >(
        &self,
        cap: Option<usize>,
    ) -> (Sender<T>, Receiver<T>) {
        if let Ok(chan) = self.get_channel::<T>() {
            debug!("Returning already registered channel");
            return chan;
        }

        let tid_hash: StitchRegistryKey = StitchMessage::hash_type::<T>();

        let (serializer_sender, serializer_receiver): (Sender<T>, Receiver<T>) =
            channel_factory(cap);
        let (stream_writer_sender, _) = self.stream_writer_chan();

        let (deserializer_sender, deserializer_receiver) = unbounded::<StitchMessage>();
        let (user_sender, user_receiver): (Sender<T>, Receiver<T>) = channel_factory(cap);

        self.registry().insert(
            tid_hash,
            Arc::new(StitchRegistryEntry::new(
                (serializer_sender.clone(), serializer_receiver.clone()),
                (deserializer_sender.clone(), deserializer_receiver.clone()),
                (user_sender.clone(), user_receiver.clone()),
                task::spawn(crate::net::tasks::serialize::<T>(
                    serializer_receiver,
                    stream_writer_sender,
                )),
                task::spawn(crate::net::tasks::deserialize::<T>(
                    deserializer_receiver,
                    user_sender,
                )),
            )),
        );

        return (serializer_sender, user_receiver);
    }
}

pub struct StitchNetClientAgent {
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    registry: StitchRegistry,
    stream_writer_chan: (Sender<StitchMessage>, Receiver<StitchMessage>),
    read_task: task::JoinHandle<anyhow::Result<()>>,
    write_task: task::JoinHandle<anyhow::Result<()>>,
}

impl StitchNetClient for StitchNetClientAgent {
    fn registry(&self) -> StitchRegistry {
        self.registry.clone()
    }

    fn stream_writer_chan(&self) -> (Sender<StitchMessage>, Receiver<StitchMessage>) {
        (
            self.stream_writer_chan.0.clone(),
            self.stream_writer_chan.1.clone(),
        )
    }

    fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    fn close(self) {
        let (s, r) = self.stream_writer_chan;
        s.close();
        r.close();

        task::block_on(self.read_task.cancel());
        task::block_on(self.write_task.cancel());
    }
}
