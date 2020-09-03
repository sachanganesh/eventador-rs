mod registry;
pub mod tasks;
pub mod tcp;
pub mod tls;

use crate::net::registry::{StitchRegistry, StitchRegistryEntry, StitchRegistryKey};
use crate::{channel_factory, StitchMessage};
use async_channel::{unbounded, Receiver, Sender};
use async_std::net::SocketAddr;
use async_std::sync::{Arc, Condvar, Mutex};
use async_std::task;
use async_std::task::JoinHandle;
use dashmap::DashMap;
use log::*;

pub trait StitchClient {
    fn local_addr(&self) -> SocketAddr;
    fn peer_addr(&self) -> SocketAddr;

    fn registry(&self) -> StitchRegistry;
    fn stream_writer_chan(&self) -> (Sender<StitchMessage>, Receiver<StitchMessage>);

    fn read_task(&self) -> &JoinHandle<anyhow::Result<()>>;
    fn write_task(&self) -> &JoinHandle<anyhow::Result<()>>;

    fn read_readiness(&self) -> Arc<(Mutex<bool>, Condvar)>;
    fn write_readiness(&self) -> Arc<(Mutex<bool>, Condvar)>;
    fn is_ready(&self) -> bool;
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
        let tid_hash: StitchRegistryKey = StitchMessage::hash_type::<T>();

        if let Ok(chan) = self.get_channel::<T>() {
            debug!("Returning already registered channel");
            return chan;
        } else {
            debug!("Creating entry for type-id {} in the registry", tid_hash)
        }

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

    fn ready(&self) -> anyhow::Result<()> {
        debug!("Attempting to ready connection");

        let (read_ready, read_cvar) = &*self.read_readiness();
        let mut read_ready_handle = task::block_on(read_ready.lock());

        if *read_ready_handle == true {
            warn!("Service already in `ready` state despite call");
            return Ok(());
        }

        *read_ready_handle = true;
        read_cvar.notify_all();
        drop(read_ready_handle);

        let (write_ready, write_cvar) = &*self.write_readiness();
        let mut write_ready_handle = task::block_on(write_ready.lock());
        *write_ready_handle = true;
        write_cvar.notify_all();
        drop(write_ready_handle);

        info!(
            "Notified waiting read ({}) and write ({}) tasks of ready state",
            self.read_task().task().id(),
            self.write_task().task().id()
        );

        Ok(())
    }
}

pub struct StitchNetClient {
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    registry: StitchRegistry,
    stream_writer_chan: (Sender<StitchMessage>, Receiver<StitchMessage>),
    read_readiness: Arc<(Mutex<bool>, Condvar)>,
    write_readiness: Arc<(Mutex<bool>, Condvar)>,
    read_task: task::JoinHandle<anyhow::Result<()>>,
    write_task: task::JoinHandle<anyhow::Result<()>>,
}

impl StitchClient for StitchNetClient {
    fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    fn registry(&self) -> StitchRegistry {
        self.registry.clone()
    }

    fn stream_writer_chan(&self) -> (Sender<StitchMessage>, Receiver<StitchMessage>) {
        (
            self.stream_writer_chan.0.clone(),
            self.stream_writer_chan.1.clone(),
        )
    }

    fn read_task(&self) -> &JoinHandle<anyhow::Result<()>> {
        &self.read_task
    }

    fn write_task(&self) -> &JoinHandle<anyhow::Result<()>> {
        &self.write_task
    }

    fn read_readiness(&self) -> Arc<(Mutex<bool>, Condvar)> {
        self.read_readiness.clone()
    }

    fn write_readiness(&self) -> Arc<(Mutex<bool>, Condvar)> {
        self.write_readiness.clone()
    }

    fn is_ready(&self) -> bool {
        let (ready, _) = &*self.read_readiness;
        let handle = task::block_on(ready.lock());
        *handle
    }

    fn close(self) {
        let (s, r) = self.stream_writer_chan;
        s.close();
        r.close();

        task::block_on(self.read_task.cancel());
        task::block_on(self.write_task.cancel());
    }
}

type ServerRegistry = Arc<DashMap<SocketAddr, Arc<StitchNetClient>>>;

pub trait StitchServer {
    fn accept_loop_task(&self) -> &task::JoinHandle<anyhow::Result<()>>;

    fn close(self);
}

#[allow(dead_code)]
pub struct StitchNetServer {
    registry: ServerRegistry,
    connections_chan: (Sender<Arc<StitchNetClient>>, Receiver<Arc<StitchNetClient>>),
    accept_loop_task: task::JoinHandle<anyhow::Result<()>>,
}

impl StitchServer for StitchNetServer {
    fn accept_loop_task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.accept_loop_task
    }

    fn close(self) {
        self.connections_chan.0.close();
        self.connections_chan.1.close();

        task::block_on(self.accept_loop_task.cancel());
    }
}
