use async_std::prelude::*;
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use crossbeam_channel::{Receiver, Sender, unbounded, bounded};

pub struct BiDirectionalTcpChannel<T>
    where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    rx_chan: (Sender<T>, Receiver<T>),
    tx_chan: (Sender<T>, Receiver<T>),
    pub reader:  task::JoinHandle<anyhow::Result<()>>,
    pub writer:  task::JoinHandle<anyhow::Result<()>>,
}

impl<T> BiDirectionalTcpChannel<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded<A: ToSocketAddrs>(ip_addrs: A) -> Result<Self> {
        Self::from(ip_addrs, unbounded(), unbounded())
    }

    pub fn bounded<A: ToSocketAddrs>(ip_addrs: A, outgoing_bound: Option<usize>, incoming_bound: Option<usize>) -> Result<Self> {
        let outgoing_chan = if let Some(bound) = outgoing_bound {
            bounded(bound)
        } else {
            unbounded()
        };

        let incoming_chan = if let Some(bound) = incoming_bound {
            bounded(bound)
        } else {
            unbounded()
        };

        Self::from(ip_addrs, outgoing_chan, incoming_chan)
    }

    pub fn from<A: ToSocketAddrs>(ip_addrs: A, outgoing_chan: (Sender<T>, Receiver<T>), incoming_chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let read_stream  = task::block_on(TcpStream::connect(ip_addrs))?;
        let write_stream = read_stream.clone();

        write_stream.set_nodelay(true)?;

        let _receiver = outgoing_chan.1.clone();
        let _sender   = incoming_chan.0.clone();

        Ok(BiDirectionalTcpChannel {
            rx_chan: incoming_chan,
            tx_chan: outgoing_chan,
            reader:  task::spawn(Self::read_from_stream(read_stream, _sender)),
            writer:  task::spawn(Self::write_to_stream(_receiver, write_stream))
        })
    }

    pub fn channel(&self) -> (Sender<T>, Receiver<T>) {
        (self.tx_chan.0.clone(), self.rx_chan.1.clone())
    }

    fn tcp_channel(&self) -> (&Sender<T>, &Receiver<T>) {
        (&self.rx_chan.0, &self.tx_chan.1)
    }

    pub fn close(self) {
        task::block_on(self.reader.cancel());
        task::block_on(self.writer.cancel());
    }

    async fn read_from_stream(mut input: TcpStream, output: Sender<T>) -> anyhow::Result<()>
    where T: for<'de> serde::de::Deserialize<'de> {
        use std::convert::TryInto;
        use bytes::{Buf, BytesMut};

        const BUFFER_SIZE: usize = 8192;

        let mut buffer = BytesMut::new();
        buffer.resize(BUFFER_SIZE, 0);

        let mut pending: Option<BytesMut> = None;

        while let Ok(mut bytes_read_raw) = input.read(&mut buffer).await {
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
        }

        Ok(())
    }

    async fn write_to_stream(input: Receiver<T>, mut output: TcpStream) -> anyhow::Result<()>
    where T: serde::ser::Serialize {
        loop {
            while let Ok(t) = input.try_recv() {
                if let Ok(data) = bincode::serialize(&t) {
                    if let Ok(_) = output.write_all(&data).await {
                        output.flush();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        Ok(())
    }
}
