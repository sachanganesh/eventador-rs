use std::io::Write;
use std::time::SystemTime;

use async_std::prelude::*;
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use crossbeam_channel::{Receiver, Sender, unbounded, bounded};

pub const MAX_MESSAGES: usize = 100;

pub struct BiDirectionalTcpChannel<T>
    where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    rx_chan: (Sender<T>, Receiver<T>),
    tx_chan: (Sender<T>, Receiver<T>),
    pub reader:  task::JoinHandle<Vec<SystemTime>>,
    pub writer:  task::JoinHandle<Vec<SystemTime>>,
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

        // let read_stream = std::net::TcpStream::connect(ip_addrs)?;
        // let write_stream = read_stream.try_clone()?;
        // let write_stream = TcpStream::from(write_stream);


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

    async fn read_from_stream(mut input: TcpStream, output: Sender<T>) -> Vec<SystemTime>
    where T: for<'de> serde::de::Deserialize<'de> {
        use bytes::{Buf, BytesMut};

        let mut arr: Vec<SystemTime> = Vec::with_capacity(MAX_MESSAGES);
        let mut i = 0;

        let mut buffer = BytesMut::new();
        buffer.resize(8192, 0);

        while let Ok(bytes_read_raw) = input.read(&mut buffer).await {
            let mut bytes_read: u64 = bytes_read_raw as u64;
            while bytes_read > 0 {
                // print!("r");
                // std::io::stdout().flush();

                if let Ok(data) = bincode::deserialize(&buffer) {
                    if let Ok(serialized_size_64) = bincode::serialized_size(&data) {
                        let serialized_size = serialized_size_64 as usize;
                        buffer.advance(serialized_size);

                        bytes_read -= serialized_size_64; // overflow errors!! @todo
                        buffer.resize(8192, 0);
                    } else {
                        break;
                    }

                    arr.push(SystemTime::now());
                    if let Err(_) = output.try_send(data) {
                        break;
                    } else {
                        i += 1;

                        if i == MAX_MESSAGES {
                            break;
                        }
                    }
                }
            }

            // task::yield_now().await

            if i == MAX_MESSAGES {
                break;
            }
        }

        return arr;
    }

    async fn write_to_stream(input: Receiver<T>, mut output: TcpStream) -> Vec<SystemTime>
    where T: serde::ser::Serialize {
        const MAX_REDUCTIONS: usize = 2000;
        let mut reductions = 0;

        let mut arr: Vec<SystemTime> = Vec::with_capacity(MAX_MESSAGES);
        let mut i = 0;

        loop {
            while let Ok(t) = input.try_recv() {
                reductions += 1;

                // print!("w"); //, bincode::serialized_size(&t).expect("todo"));
                // std::io::stdout().flush();

                arr.push(SystemTime::now());
                if let Ok(data) = bincode::serialize(&t) {
                    if let Err(_) = output.write_all(&data).await {
                        break;
                    } else {
                        i += 1;

                        if i == MAX_MESSAGES {
                            break;
                        }
                    }
                } else {
                    break;
                }

                output.flush();

                if reductions == MAX_REDUCTIONS {
                    reductions = 0;
                    break;
                }
            }

            // task::yield_now().await

            if i == MAX_MESSAGES {
                break;
            }
        }

        return arr;
    }
}
