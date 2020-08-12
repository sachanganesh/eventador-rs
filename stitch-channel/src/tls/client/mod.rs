pub(crate) mod bidi;
pub(crate) mod read;
pub(crate) mod write;

pub use async_tls;
pub use rustls;

use async_std::net::TcpStream;
use async_tls::client::TlsStream;
use async_channel::{Receiver, Sender};
use futures_util::{AsyncWriteExt, AsyncReadExt, io::{ReadHalf, WriteHalf}};
use log::*;

const BUFFER_SIZE: usize = 8192;

async fn read_from_stream<T>(mut input: ReadHalf<TlsStream<TcpStream>>, output: Sender<T>) -> anyhow::Result<()>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    use std::convert::TryInto;
    use bincode::config::Options;
    use bytes::{Buf, BytesMut};

    let converter = bincode::options().with_limit(1_000_000);

    let mut buffer = BytesMut::new();
    buffer.resize(BUFFER_SIZE, 0);

    let mut pending: Option<BytesMut> = None;

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
                    debug!("{} bytes from TLS stream still unprocessed", bytes_read_raw);

                    let bytes = std::io::Cursor::new(buffer.as_ref());
                    match converter.deserialize_from(bytes) {
                        Ok(data) => {
                            if let Ok(serialized_size) = converter.serialized_size(&data) {
                                buffer.advance(serialized_size.try_into()?);
                                bytes_read = bytes_read.saturating_sub(serialized_size);

                                debug!("Consumed message of size {} bytes, {} bytes still unprocessed", serialized_size, bytes_read);
                            } else {
                                warn!("Could not determine serialized size of already parsed data");
                                break;
                            }

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

            Err(err) => {
                return Err(anyhow::Error::from(err))
            }
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
                if let Ok(data) = bincode::serialize(&msg) {
                    // @todo provide retry mechanism with backoff
                    if let Ok(_) = output.write_all(&data).await {
                        debug!("Wrote {} bytes to TLS stream", data.len());
                        output.flush();
                    }
                }
            },

            Err(err) => return Err(anyhow::Error::from(err))
        }
    }
}
