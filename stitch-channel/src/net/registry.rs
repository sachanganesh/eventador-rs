use crate::net::*;
use async_std::sync::RwLock;
use async_std::task;
use async_std::task::JoinHandle;
use std::any::Any;
use std::sync::Arc;

pub(crate) type StitchRegistry = Arc<RwLock<HashMap<StitchRegistryKey, Arc<StitchRegistryEntry>>>>;

pub(crate) type StitchRegistryKey = u64;

type GenericField = Box<dyn Any + Send + Sync>;
pub(crate) struct StitchRegistryEntry {
    serializer_sender: GenericField,
    serializer_receiver: GenericField,

    deserializer_sender: Sender<StitchMessage>,
    deserializer_receiver: Receiver<StitchMessage>,

    user_sender: GenericField,
    user_receiver: GenericField,

    serializer_task: task::JoinHandle<Result<(), anyhow::Error>>,
    deserializer_task: task::JoinHandle<Result<(), anyhow::Error>>,
}

impl StitchRegistryEntry {
    pub fn new<T: 'static + Send + Sync>(
        serializer_chan: (Sender<Box<T>>, Receiver<Box<T>>),
        deserializer_chan: (Sender<StitchMessage>, Receiver<StitchMessage>),
        user_chan: (Sender<Box<T>>, Receiver<Box<T>>),

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

    // fn downcast_sender<T: 'static + Send + Sync>(
    //     downcastable_sender: &Sender<Box<dyn Any + Send + Sync>>,
    // ) -> Result<Sender<T>, anyhow::Error> {
    //     if let Some(sender) = downcastable_sender.downcast_ref::<Sender<Box<T>>>() {
    //         return Ok(sender.clone());
    //     }
    //
    //     Err(anyhow::Error::msg(
    //         "could not downcast sender based on type parameter",
    //     ))
    // }

    // fn downcast_receiver<T: 'static + Send + Sync>(
    //     downcastable_receiver: &Box<dyn Any + Send + Sync>,
    // ) -> Result<Receiver<T>, anyhow::Error> {
    //     if let Some(receiver) = downcastable_receiver.downcast_ref::<Receiver<T>>() {
    //         return Ok(receiver.clone());
    //     }
    //
    //     Err(anyhow::Error::msg(
    //         "could not downcast receiver based on type parameter",
    //     ))
    // }
    //
    // pub fn serialize_sender<T: 'static + Send + Sync>(&self) -> Result<Sender<Box<T>>, anyhow::Error> {
    //     Self::downcast_sender(&self.serialize_sender)
    // }
    //
    // pub fn serialize_receiver<T: 'static + Send + Sync>(
    //     &self,
    // ) -> Result<Receiver<T>, anyhow::Error> {
    //     Self::downcast_receiver(&self.serialize_receiver)
    // }
    //
    pub fn deserializer_sender(&self) -> Sender<StitchMessage> {
        self.deserializer_sender.clone()
    }
    //
    // pub fn deserialize_receiver<T: 'static + Send + Sync>(
    //     &self,
    // ) -> Result<Receiver<T>, anyhow::Error> {
    //     Self::downcast_receiver(&self.deserialize_receiver)
    // }
    //
    // pub fn external_chan<T: 'static + Send + Sync>(
    //     &self,
    // ) -> Result<(Sender<Box<T>>, Receiver<Box<T>>), anyhow::Error> {
    //     match self.serialize_sender() {
    //         Ok(sender) => match self.deserialize_receiver() {
    //             Ok(receiver) => Ok((sender, receiver)),
    //             Err(err) => Err(err),
    //         },
    //
    //         Err(err) => Err(err),
    //     }
    // }
}
