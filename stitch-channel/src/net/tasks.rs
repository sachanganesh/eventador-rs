use crate::net::registry::{StitchRegistry, StitchRegistryKey};
use crate::StitchMessage;
use async_channel::{Receiver, Sender};
use async_std::prelude::*;
use async_std::sync::{Arc, Condvar, Mutex};
use bytes::{Buf, BytesMut};
use futures_util::io::{AsyncRead, AsyncWrite};
use log::*;
use rmp_serde::{decode, encode};
use serde::Deserialize;
use std::io::Cursor;

const BUFFER_SIZE: usize = 8192;

pub(crate) async fn read_from_stream<R>(
    registry: StitchRegistry,
    mut input: R,
    readiness: Arc<(Mutex<bool>, Condvar)>,
) -> anyhow::Result<()>
where
    R: AsyncRead + std::marker::Unpin,
{
    use std::convert::TryInto;

    let (lock, cvar) = &*readiness;
    let mut ready = lock.lock().await;
    while !*ready {
        ready = cvar.wait(ready).await;
    }

    let mut buffer = BytesMut::new();
    buffer.resize(BUFFER_SIZE, 0);

    let mut pending: Option<BytesMut> = None;

    debug!("Starting read loop for the network connection");
    loop {
        trace!("Reading from the stream");
        match input.read(&mut buffer).await {
            Ok(mut bytes_read_raw) => {
                if bytes_read_raw > 0 {
                    debug!("Read {} bytes from the network stream", bytes_read_raw)
                }

                if let Some(mut pending_buf) = pending.take() {
                    debug!("Prepending broken data ({} bytes) encountered from earlier read of network stream", pending_buf.len());
                    bytes_read_raw += pending_buf.len();

                    pending_buf.unsplit(buffer);
                    buffer = pending_buf;
                }

                let mut bytes_read: u64 = bytes_read_raw.try_into()?;
                while bytes_read > 0 {
                    debug!("{} bytes from network stream still unprocessed", bytes_read);

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

                            let tid: StitchRegistryKey = data.type_id;

                            match registry.get(&tid) {
                                Some(entry) => {
                                    debug!("Looked up registered channel for type-id {}", tid);
                                    let sender = entry.deserializer_sender();

                                    debug!("Sending deserialized data to channel");
                                    if let Err(err) = sender.send(data).await {
                                        error!("Encountered error while sending data to channel from network stream: {:#?}", err);
                                        return Err(anyhow::Error::from(err));
                                    }
                                }

                                // got an external message that has not yet been registered for internal consumption
                                None => {
                                    // @todo need to create a channel on a need-by-need basis
                                    error!(
                                        "Could not find entry for type-id {} in the registry",
                                        tid
                                    );
                                    return Err(anyhow::Error::msg(format!(
                                        "failed to find channels of type-id {} registered",
                                        tid
                                    )));
                                }
                            }
                        }

                        Err(err) => {
                            warn!(
                                "Could not deserialize data from the network connection: {:#?}",
                                err
                            );
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

pub(crate) async fn write_to_stream<T, W>(
    input: Receiver<T>,
    mut output: W,
    readiness: Arc<(Mutex<bool>, Condvar)>,
) -> anyhow::Result<()>
where
    T: 'static + serde::ser::Serialize,
    W: AsyncWrite + std::marker::Unpin,
{
    let (lock, cvar) = &*readiness;
    let mut ready = lock.lock().await;
    while !*ready {
        ready = cvar.wait(ready).await;
    }

    debug!("Starting write loop for the network connection");
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
                            debug!("Wrote {} bytes to network stream", buffer.len());
                            output.flush().await?;
                        }

                        Err(err) => error!("Could not write data to network stream: {}", err),
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
    let tid_hash: StitchRegistryKey = StitchMessage::hash_type::<T>();

    debug!("Starting serialize loop for type-id {}", tid_hash);
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
    output: Sender<T>,
) -> anyhow::Result<()>
where
    T: Send + Sync + for<'de> serde::de::Deserialize<'de>,
{
    let tid_hash: StitchRegistryKey = StitchMessage::hash_type::<T>();

    debug!("Starting deserialize loop for type-id {}", tid_hash);
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
                        if let Err(err) = output.send(data).await {
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
