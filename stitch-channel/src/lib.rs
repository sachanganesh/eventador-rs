pub mod tcp;
pub mod tls;

pub use async_channel::{Sender, Receiver};

use async_channel::{bounded, unbounded};
use async_std::prelude::*;
use bytes::{Buf, BytesMut};
use futures_util::io::{AsyncRead, AsyncWrite};
use log::*;
use rmp_serde::{decode, encode};
use serde::Deserialize;
use std::io::Cursor;

const BUFFER_SIZE: usize = 8192;

pub(crate) async fn read_from_stream<R, T>(mut input: R, output: Sender<T>) -> anyhow::Result<()>
where
    T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
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
                            let serialized_size = decoder.position();
                            buffer.advance(serialized_size.try_into()?);
                            bytes_read -= serialized_size;
                            debug!("Consumed a message, {} bytes still unprocessed", bytes_read);

                            debug!("Sending deserialized data to channel");
                            if let Err(err) = output.send(data).await {
                                error!("Encountered error while sending data to channel from TCP stream: {:#?}", err);
                                return Err(anyhow::Error::from(err));
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
    T: 'static + Send + Sync + serde::ser::Serialize,
    W: AsyncWrite + std::marker::Unpin,
{
    debug!("Starting write loop for TCP connection");
    loop {
        trace!("Waiting for writable data that will be sent to the stream");
        match input.recv().await {
            Ok(msg) => {
                debug!("Received message from channel to be written stream");
                let mut buffer = Vec::new();
                let mut serializer = encode::Serializer::new(&mut buffer);

                match msg.serialize(&mut serializer) {
                    Ok(_) => match output.write_all(buffer.as_slice()).await {
                        Ok(_) => {
                            debug!("Wrote {:?} as {} bytes to TCP stream", buffer, buffer.len());
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

pub(crate) fn channel_factory<
    T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
>(
    mut bound: Option<usize>,
) -> (Sender<T>, Receiver<T>) {
    if let Some(bound) = bound.take() {
        bounded(bound)
    } else {
        unbounded()
    }
}
