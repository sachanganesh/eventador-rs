pub(crate) mod bidi;
pub(crate) mod read;
pub(crate) mod write;

pub use async_tls;
pub use rustls;

use std::io::Cursor;
use async_std::net::TcpStream;
use async_tls::client::TlsStream;
use async_channel::{Receiver, Sender};
use bytes::{Buf, BytesMut};
use futures_util::{AsyncWriteExt, AsyncReadExt, io::{ReadHalf, WriteHalf}};
use log::*;
use rmp_serde::{encode, decode};
use serde::Deserialize;

const BUFFER_SIZE: usize = 8192;

async fn read_from_stream<T>(mut input: ReadHalf<TlsStream<TcpStream>>, output: Sender<T>) -> anyhow::Result<()>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    use std::convert::TryInto;

    let mut buffer = BytesMut::new();
    buffer.resize(BUFFER_SIZE, 0);

    let mut pending: Option<BytesMut> = None;
    let mut prev_position: Option<u64> = None;

    debug!("Starting read loop for TLS connection");
    loop {
        trace!("Reading from the stream");
        match input.read(&mut buffer).await {
            Ok(mut bytes_read_raw) => {
                if bytes_read_raw > 0 {
                    debug!("Read {} bytes from TLS stream", bytes_read_raw)
                }

                if let Some(mut pending_buf) = pending.take() {
                    debug!("Prepending broken data ({} bytes) encountered from earlier read of TLS stream", pending_buf.len());
                    bytes_read_raw += pending_buf.len();

                    pending_buf.unsplit(buffer);
                    buffer = pending_buf;
                }

                let mut bytes_read: u64 = bytes_read_raw.try_into()?;
                while bytes_read > 0 {
                    debug!("{} bytes from TLS stream still unprocessed", bytes_read);

                    let buf_read = Cursor::new(buffer.as_ref());

                    let mut decoder = decode::Deserializer::new(buf_read);
                    prev_position = None;

                    match Deserialize::deserialize(&mut decoder) {
                        Ok(data) => {
                            let serialized_size = match prev_position {
                                Some(pos) => decoder.position() - pos,
                                None => decoder.position()
                            };
                            buffer.advance(serialized_size.try_into()?);

                            bytes_read -= serialized_size;
                            debug!("Consumed a message, {} bytes still unprocessed", bytes_read);

                            debug!("Sending deserialized data to channel");
                            if let Err(err) = output.send(data).await {
                                error!("Encountered error while sending data to channel from TLS stream: {:#?}", err);
                                return Err(anyhow::Error::from(err))
                            }
                        },

                        Err(err) => {
                            error!("Encountered error while deserializing data from TLS connection: {:#?}", err);
                            pending = Some(buffer);
                            buffer = BytesMut::new();
                            break;
                        }
                    }
                }

                buffer.resize(BUFFER_SIZE, 0);
            },

            Err(err) => return Err(anyhow::Error::from(err))
        }
    }
}

async fn write_to_stream<T>(input: Receiver<T>, mut output: WriteHalf<TlsStream<TcpStream>>) -> anyhow::Result<()>
where T: 'static + Send + Sync + serde::ser::Serialize {
    debug!("Starting write loop for TLS connection");
    loop {
        trace!("Waiting for writable data that will be sent to the stream");
        match input.recv().await {
            Ok(msg) => {
                debug!("Received message from channel to be written stream");
                let mut buffer = Vec::new();
                let mut serializer = encode::Serializer::new(&mut buffer);

                match msg.serialize(&mut serializer) {
                    Ok(_) => {
                        match output.write_all(buffer.as_slice()).await {
                            Ok(_) => {
                                debug!("Wrote {:?} as {} bytes to TLS stream", buffer, buffer.len());
                                output.flush().await;
                            },

                            Err(err) => {
                                error!("Could not write data to TLS stream: {}", err)
                            }
                        }
                    },

                    Err(err) => {
                        error!("Could not serialize message: {}", err);
                    }
                }
            },

            Err(err) => return Err(anyhow::Error::from(err))
        }
    }
}
