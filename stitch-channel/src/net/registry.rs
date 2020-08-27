use crate::net::*;
use async_std::sync::RwLock;
use async_std::task;
use async_std::task::JoinHandle;
use std::any::Any;
use std::sync::Arc;

pub(crate) type StitchRegistry = Arc<RwLock<HashMap<StitchRegistryKey, Arc<StitchRegistryEntry>>>>;

pub(crate) type StitchRegistryKey = u64;

pub(crate) struct StitchRegistryEntry {
    serialize_sender: Box<dyn Any + Send + Sync>,
    serialize_receiver: Box<dyn Any + Send + Sync>,
    deserialize_sender: Box<dyn Any + Send + Sync>,
    deserialize_receiver: Box<dyn Any + Send + Sync>,
    serialize_task: task::JoinHandle<Result<(), anyhow::Error>>,
    deserialize_task: task::JoinHandle<Result<(), anyhow::Error>>,
}

impl StitchRegistryEntry {
    pub fn new<T: 'static + Send + Sync>(
        serialize_chan: (Sender<T>, Receiver<T>),
        deserialize_chan: (Sender<T>, Receiver<T>),
        serialize_task: JoinHandle<Result<(), anyhow::Error>>,
        deserialize_task: JoinHandle<Result<(), anyhow::Error>>,
    ) -> StitchRegistryEntry {
        StitchRegistryEntry {
            serialize_sender: Box::new(serialize_chan.0),
            serialize_receiver: Box::new(serialize_chan.1),
            deserialize_sender: Box::new(deserialize_chan.0),
            deserialize_receiver: Box::new(deserialize_chan.1),
            serialize_task,
            deserialize_task,
        }
    }

    fn downcast_sender<T: 'static + Send + Sync>(
        downcastable_sender: &Box<dyn Any + Send + Sync>,
    ) -> Result<Sender<T>, anyhow::Error> {
        if let Some(sender) = downcastable_sender.downcast_ref::<Sender<T>>() {
            return Ok(sender.clone());
        }

        Err(anyhow::Error::msg(
            "could not downcast based on implicit type parameter",
        ))
    }

    fn downcast_receiver<T: 'static + Send + Sync>(
        downcastable_receiver: &Box<dyn Any + Send + Sync>,
    ) -> Result<Receiver<T>, anyhow::Error> {
        if let Some(receiver) = downcastable_receiver.downcast_ref::<Receiver<T>>() {
            return Ok(receiver.clone());
        }

        Err(anyhow::Error::msg(
            "could not downcast based on implicit type parameter",
        ))
    }

    pub fn serialize_sender<T: 'static + Send + Sync>(&self) -> Result<Sender<T>, anyhow::Error> {
        Self::downcast_sender(&self.serialize_sender)
    }

    pub fn serialize_receiver<T: 'static + Send + Sync>(
        &self,
    ) -> Result<Receiver<T>, anyhow::Error> {
        Self::downcast_receiver(&self.serialize_receiver)
    }

    pub fn deserialize_sender<T: 'static + Send + Sync>(&self) -> Result<Sender<T>, anyhow::Error> {
        Self::downcast_sender(&self.deserialize_sender)
    }

    pub fn deserialize_receiver<T: 'static + Send + Sync>(
        &self,
    ) -> Result<Receiver<T>, anyhow::Error> {
        Self::downcast_receiver(&self.deserialize_receiver)
    }

    pub fn external_chan<T: 'static + Send + Sync>(
        &self,
    ) -> Result<(Sender<Box<T>>, Receiver<Box<T>>), anyhow::Error> {
        match self.serialize_sender() {
            Ok(sender) => match self.deserialize_receiver() {
                Ok(receiver) => Ok((sender, receiver)),
                Err(err) => Err(err),
            },

            Err(err) => Err(err),
        }
    }
}
