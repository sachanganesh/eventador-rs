pub(crate) mod bidi;
pub(crate) mod read;
pub(crate) mod write;
pub(crate) mod server;

pub use bidi::*;
pub use read::*;
pub use write::*;
pub use server::*;

use async_std::prelude::*;
use async_std::net::TcpStream;
use async_std::task;
use async_channel::{Receiver, Sender};
use log::*;


const BUFFER_SIZE: usize = 8192;

async fn read_from_stream<T>(mut input: TcpStream, output: Sender<T>) -> anyhow::Result<()>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    use std::convert::TryInto;
    use bytes::{Buf, BytesMut};

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
                    debug!("{} bytes from TCP stream still unprocessed", bytes_read_raw);

                    match bincode::deserialize(&buffer) {
                        Ok(data) => {
                            if let Ok(serialized_size) = bincode::serialized_size(&data) {
                                buffer.advance(serialized_size.try_into()?);
                                bytes_read = bytes_read.saturating_sub(serialized_size);

                                debug!("Consumed message of size {} bytes, {} bytes still unprocessed", serialized_size, bytes_read);
                            } else {
                                warn!("Could not determine serialized size of already parsed data");
                                break;
                            }

                            if let Err(err) = output.send(data).await {
                                error!("Encountered error while sending data to channel from TCP stream: {:#?}", err);
                                return Err(anyhow::Error::from(err))
                            }
                        },

                        Err(err) => {
                            error!("Encountered error while deserializing data from TCP connection: {:#?}", err);
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

async fn write_to_stream<T>(input: Receiver<T>, mut output: TcpStream) -> anyhow::Result<()>
where T: 'static + Send + Sync + serde::ser::Serialize {
    debug!("Starting write loop for TCP connection");
    loop {
        trace!("Waiting for writable data that will be sent to the stream");
        match input.recv().await {
            Ok(msg) => {
                if let Ok(data) = bincode::serialize(&msg) {
                    if let Ok(_) = output.write_all(&data).await {
                        debug!("Wrote {} bytes to TCP stream", data.len());
                        output.flush().await;
                    }
                }
            },

            Err(err) => return Err(anyhow::Error::from(err))
        }
    }
}
