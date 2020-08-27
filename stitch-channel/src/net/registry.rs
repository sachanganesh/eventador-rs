use crate::net::*;
use async_std::task;
use async_std::task::JoinHandle;
use dashmap::DashMap;
use std::any::Any;
use std::sync::Arc;

pub(crate) type StitchRegistry = Arc<DashMap<StitchRegistryKey, Arc<StitchRegistryEntry>>>;

pub(crate) fn new_stitch_registry() -> StitchRegistry {
    Arc::new(DashMap::new())
}

pub(crate) type StitchRegistryKey = u64;

type GenericChannelEndpoint = Box<dyn Any + Send + Sync>;
pub(crate) struct StitchRegistryEntry {
    serializer_sender: GenericChannelEndpoint,
    serializer_receiver: GenericChannelEndpoint,

    deserializer_sender: Sender<StitchMessage>,
    deserializer_receiver: Receiver<StitchMessage>,

    user_sender: GenericChannelEndpoint,
    user_receiver: GenericChannelEndpoint,

    serializer_task: task::JoinHandle<Result<(), anyhow::Error>>,
    deserializer_task: task::JoinHandle<Result<(), anyhow::Error>>,
}

impl StitchRegistryEntry {
    pub fn new<T: 'static + Send + Sync>(
        serializer_chan: (Sender<T>, Receiver<T>),
        deserializer_chan: (Sender<StitchMessage>, Receiver<StitchMessage>),
        user_chan: (Sender<T>, Receiver<T>),

        serializer_task: JoinHandle<Result<(), anyhow::Error>>,
        deserializer_task: JoinHandle<Result<(), anyhow::Error>>,
    ) -> StitchRegistryEntry {
        StitchRegistryEntry {
            serializer_sender: Box::new(serializer_chan.0),
            serializer_receiver: Box::new(serializer_chan.1),

            deserializer_sender: deserializer_chan.0,
            deserializer_receiver: deserializer_chan.1,

            user_sender: Box::new(user_chan.0),
            user_receiver: Box::new(user_chan.1),

            serializer_task,
            deserializer_task,
        }
    }

    pub fn deserializer_sender(&self) -> Sender<StitchMessage> {
        self.deserializer_sender.clone()
    }

    fn downcast_sender<T: 'static + Send + Sync>(
        downcastable_sender: &GenericChannelEndpoint,
    ) -> Result<Sender<Box<T>>, anyhow::Error> {
        if let Some(sender) = downcastable_sender.downcast_ref::<Sender<Box<T>>>() {
            return Ok(sender.clone());
        }

        Err(anyhow::Error::msg(
            "could not downcast sender based on type parameter",
        ))
    }

    fn downcast_receiver<T: 'static + Send + Sync>(
        downcastable_receiver: &GenericChannelEndpoint,
    ) -> Result<Receiver<Box<T>>, anyhow::Error> {
        if let Some(receiver) = downcastable_receiver.downcast_ref::<Receiver<Box<T>>>() {
            return Ok(receiver.clone());
        }

        Err(anyhow::Error::msg(
            "could not downcast receiver based on type parameter",
        ))
    }

    pub fn user_facing_chan<T: 'static + Send + Sync>(
        &self,
    ) -> Result<(Sender<Box<T>>, Receiver<Box<T>>), anyhow::Error> {
        match Self::downcast_sender(&self.serializer_sender) {
            Ok(sender) => match Self::downcast_receiver(&self.user_receiver) {
                Ok(receiver) => Ok((sender, receiver)),

                Err(err) => Err(err),
            },

            Err(err) => Err(err),
        }
    }
}
