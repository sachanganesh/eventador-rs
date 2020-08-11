pub(crate) mod bidi;
pub(crate) mod read;
pub(crate) mod write;

pub use bidi::*;
pub use read::*;
pub use write::*;

use async_std::prelude::*;
use async_std::net::TcpStream;
use crossbeam_channel::{Receiver, Sender};


const BUFFER_SIZE: usize = 8192;

async fn read_from_stream<T>(mut input: TcpStream, output: Sender<T>) -> anyhow::Result<()>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    use std::convert::TryInto;
    use bytes::{Buf, BytesMut};

    let mut buffer = BytesMut::new();
    buffer.resize(BUFFER_SIZE, 0);

    let mut pending: Option<BytesMut> = None;

    loop {
        match input.read(&mut buffer).await {
            Ok(mut bytes_read_raw) => {
                if let Some(mut pending_buf) = pending.take() {
                    bytes_read_raw += pending_buf.len();

                    pending_buf.unsplit(buffer);
                    buffer = pending_buf;
                }

                let mut bytes_read: u64 = bytes_read_raw.try_into()?;
                while bytes_read > 0 {
                    match bincode::deserialize(&buffer) {
                        Ok(data) => {
                            if let Ok(serialized_size) = bincode::serialized_size(&data) {
                                buffer.advance(serialized_size.try_into()?);
                                bytes_read = bytes_read.saturating_sub(serialized_size);
                            } else {
                                break;
                            }

                            if let Err(err) = output.send(data) {
                                return Err(anyhow::Error::from(err))
                            }
                        },

                        Err(_err) => {
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
    loop {
        match input.recv() {
            Ok(t) => {
                if let Ok(data) = bincode::serialize(&t) {
                    if let Ok(_) = output.write_all(&data).await {
                        output.flush();
                    }
                }
            },

            Err(err) => return Err(anyhow::Error::from(err))
        }
    }
}
