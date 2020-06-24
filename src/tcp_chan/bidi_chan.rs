use async_std::prelude::*;
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use crossbeam_channel::{Receiver, Sender, unbounded, bounded};

pub struct BiDirectionalTcpChannel<T>
    where T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    rx_chan: (Sender<T>, Receiver<T>),
    tx_chan: (Sender<T>, Receiver<T>),
    reader:  task::JoinHandle<()>,
    writer:  task::JoinHandle<()>,
}

impl<T> BiDirectionalTcpChannel<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded<A: std::net::ToSocketAddrs>(ip_addrs: A) -> Result<Self> {
        Self::from(ip_addrs, unbounded(), unbounded())
    }

    pub fn bounded<A: std::net::ToSocketAddrs>(ip_addrs: A, outgoing_bound: Option<usize>, incoming_bound: Option<usize>) -> Result<Self> {
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

    pub fn from<A: std::net::ToSocketAddrs>(ip_addrs: A, outgoing_chan: (Sender<T>, Receiver<T>), incoming_chan: (Sender<T>, Receiver<T>)) -> Result<Self> {
        let read_stream = std::net::TcpStream::connect(ip_addrs)?;
        let write_stream_raw = read_stream.try_clone()?;
        let write_stream = TcpStream::from(write_stream_raw);

        // let read_stream  = task::block_on(TcpStream::connect(ip_addrs))?;
        // let write_stream = read_stream.clone();

        // let reader = BufReader::new(read_stream);

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

    async fn read_from_stream(mut input: std::net::TcpStream, output: Sender<T>)
    where T: for<'de> serde::de::Deserialize<'de> {
        use std::io::Read;

        const MAX_REDUCTIONS: usize = 2000;
        let mut reductions: usize = 0;

        let mut buffer = Vec::with_capacity(8192);

        println!("reading!");

        loop {
            if let Ok(_) = input.read_exact(buffer.as_mut_slice()) {
                if let Ok(data) = bincode::deserialize(buffer.as_slice()) {
                    println!("got some data!");
                    if let Ok(_) = output.try_send(data) {
                        continue;
                    }
                }
            }

            // println!("yielding read");
            task::yield_now().await
        }
    }

    async fn write_to_stream(input: Receiver<T>, mut output: TcpStream) {
        const MAX_REDUCTIONS: usize = 2000;
        let mut reductions = 0;

        println!("writing!");

        loop {
            while let Ok(t) = input.recv() {
                reductions += 1;

                if reductions == MAX_REDUCTIONS {
                    reductions = 0;
                    break;
                }

                if let Ok(data) = bincode::serialize(&t) {
                    if let Ok(_) = output.write_all(&data).await {
                        println!("sent data out!");
                        continue;
                    }
                }
            }

            // println!("yielding write");
            task::yield_now().await
        }
    }
}
