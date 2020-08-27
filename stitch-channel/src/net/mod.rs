mod registry;
pub mod tcp;
// pub mod tls;

pub use async_channel::{Receiver, Sender};

use crate::net::registry::StitchRegistry;
use crate::StitchMessage;
use async_std::prelude::*;
use bytes::{Buf, BytesMut};
use futures_util::io::{AsyncRead, AsyncWrite};
use log::*;
use rmp_serde::{decode, encode};
use serde::Deserialize;
use std::any::Any;
use std::collections::HashMap;
use std::io::Cursor;

const BUFFER_SIZE: usize = 8192;

pub(crate) async fn read_from_stream<R>(
    registry: StitchRegistry,
    mut input: R,
) -> anyhow::Result<()>
where
    R: AsyncRead + std::marker::Unpin,
{
    use std::convert::TryInto;

    let mut buffer = BytesMut::new();
    buffer.resize(BUFFER_SIZE, 0);

    let mut pending: Option<BytesMut> = None;

    debug!("Starting read loop for TCP connection");
    loop {
        trace!("Reading from the stream");
        match input.read(&mut buffer).await {
            Ok(mut bytes_read_raw) => {
                if bytes_read_raw > 0 {
                    debug!("Read {} bytes from TCP stream", bytes_read_raw)
                }

                if let Some(mut pending_buf) = pending.take() {
                    debug!("Prepending broken data ({} bytes) encountered from earlier read of TCP stream", pending_buf.len());
                    bytes_read_raw += pending_buf.len();

                    pending_buf.unsplit(buffer);
                    buffer = pending_buf;
                }

                let mut bytes_read: u64 = bytes_read_raw.try_into()?;
                while bytes_read > 0 {
                    debug!("{} bytes from TCP stream still unprocessed", bytes_read);

                    let buf_read = Cursor::new(buffer.as_ref());
                    let mut decoder = decode::Deserializer::new(buf_read);

                    match Deserialize::deserialize(&mut decoder) {
                        Ok(data) => {
                            let data: StitchMessage = data;

                            let serialized_size = decoder.position();
                            buffer.advance(serialized_size.try_into()?);
                            bytes_read -= serialized_size;
                            debug!(
                                "Deserialized a message, {} bytes still unprocessed",
                                bytes_read
                            );

                            let tid: u64 = data.type_id;

                            let readable_registry = registry.read().await;
                            debug!("Acquired read lock for registry");

                            match readable_registry.get(&tid) {
                                Some(entry) => {
                                    debug!("Looked up registered channel for type id {}", tid);

                                    let sender: Sender<Box<dyn Any + Send + Sync>> = entry
                                        .deserialize_sender()
                                        .expect("internal channel get call to work"); // @todo change to better error msg

                                    debug!("Sending deserialized data to channel");
                                    if let Err(err) = sender.send(Box::new(data)).await {
                                        error!("Encountered error while sending data to channel from TCP stream: {:#?}", err);
                                        debug!("Releasing write lock for registry");
                                        return Err(anyhow::Error::from(err));
                                    }
                                }

                                // got an external message that has not yet been registered for internal consumption
                                None => {
                                    return Err(anyhow::Error::msg(format!(
                                        "failed to find channels of type id {} registered",
                                        tid
                                    )));
                                }
                            }
                        }

                        Err(err) => {
                            error!("Encountered error while deserializing data from TCP connection: {:#?}", err);
                            pending = Some(buffer);
                            buffer = BytesMut::new();
                            break;
                        }
                    }
                }

                buffer.resize(BUFFER_SIZE, 0);
            }

            Err(err) => return Err(anyhow::Error::from(err)),
        }
    }
}

pub(crate) async fn write_to_stream<T, W>(input: Receiver<T>, mut output: W) -> anyhow::Result<()>
where
    T: 'static + serde::ser::Serialize,
    W: AsyncWrite + std::marker::Unpin,
{
    debug!("Starting write loop for TCP connection");
    loop {
        trace!("Waiting for writable data that will be sent to the stream");
        match input.recv().await {
            Ok(msg) => {
                debug!("Received message from channel to be written to stream");
                let mut buffer = Vec::new();
                let mut serializer = encode::Serializer::new(&mut buffer);

                match msg.serialize(&mut serializer) {
                    Ok(_) => match output.write_all(buffer.as_slice()).await {
                        Ok(_) => {
                            debug!("Wrote {} bytes to TCP stream", buffer.len());
                            output.flush().await?;
                        }

                        Err(err) => error!("Could not write data to TCP stream: {}", err),
                    },

                    Err(err) => {
                        error!("Could not serialize message: {}", err);
                    }
                }
            }

            Err(err) => return Err(anyhow::Error::from(err)),
        }
    }
}

pub(crate) async fn serialize<T>(
    input: Receiver<T>,
    output: Sender<StitchMessage>,
) -> anyhow::Result<()>
where
    T: 'static + Send + Sync + serde::ser::Serialize,
{
    let tid = StitchMessage::type_id::<T>();
    let tid_hash = StitchMessage::hash_type_id(tid);

    debug!("Starting serialize loop for type ID {}", tid_hash);
    loop {
        match input.recv().await {
            Ok(msg) => {
                debug!(
                    "Received message from {} channel to be serialized",
                    tid_hash
                );

                let mut buffer = Vec::new();
                let mut serializer = encode::Serializer::new(&mut buffer);

                if let Err(err) = msg.serialize(&mut serializer) {
                    error!("Could not serialize message: {}", err)
                } else {
                    debug!("Serialized {} data", tid_hash);
                    let stitch_msg = StitchMessage {
                        type_id: tid_hash,
                        bytes: buffer,
                    };

                    if let Err(err) = output.send(stitch_msg.into()).await {
                        error!("Could not send {} message: {}", tid_hash, err);
                        return Err(anyhow::Error::from(err));
                    } else {
                        debug!("Sent serialized {} data to stitch-channel", tid_hash);
                    }
                }
            }

            Err(err) => return Err(anyhow::Error::from(err)),
        }
    }
}

pub(crate) async fn deserialize<T: 'static>(
    input: Receiver<StitchMessage>,
    output: Sender<Box<T>>,
) -> anyhow::Result<()>
where
    T: Send + Sync + for<'de> serde::de::Deserialize<'de>,
{
    let tid = StitchMessage::type_id::<T>();
    let tid_hash = StitchMessage::hash_type_id(tid);

    debug!("Starting deserialize loop for type ID {}", tid_hash);
    loop {
        match input.recv().await {
            Ok(msg) => {
                debug!(
                    "Received message from stitch-channel to be deserialized to {}",
                    tid_hash
                );

                let buf_read = Cursor::new(msg.bytes);
                let mut decoder = decode::Deserializer::new(buf_read);

                match Deserialize::deserialize(&mut decoder) {
                    Ok(data) => {
                        if let Err(err) = output.send(Box::new(data)).await {
                            error!("Could not send {} message: {}", tid_hash, err);
                            return Err(anyhow::Error::from(err));
                        } else {
                            debug!("Sending deserialized {} data to channel", tid_hash);
                        }
                    }

                    Err(err) => {
                        error!("Could not deserialize {} message: {}", tid_hash, err);
                        return Err(anyhow::Error::from(err));
                    }
                }
            }

            Err(err) => {
                error!("Could not receive message from {}", tid_hash);
                return Err(anyhow::Error::from(err));
            }
        }
    }
}
