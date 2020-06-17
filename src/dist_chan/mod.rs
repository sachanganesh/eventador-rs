use std::io::prelude::*;
use std::net::SocketAddr;
use std::net::TcpStream;

use crossbeam_channel::{Receiver, Sender, unbounded, bounded};
use mio::{Events, Interest, Poll, Token};
use std::thread::JoinHandle;

pub struct BiDirectionalTcpChannel<T> {
    rx_chan: (Sender<T>, Receiver<T>),
    tx_chan: (Sender<T>, Receiver<T>),
}

impl<T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>> BiDirectionalTcpChannel<T> {
    pub fn new(ip_addr: SocketAddr) -> Result<Self, anyhow::Error> {
        Self::from(ip_addr, unbounded(), unbounded())
    }

    pub fn new_bounded(ip_addr: SocketAddr, outgoing_bound: Option<usize>, incoming_bound: Option<usize>) -> Result<Self, anyhow::Error> {
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

        Self::from(ip_addr, outgoing_chan, incoming_chan)
    }

    pub fn from(ip_addr: SocketAddr, outgoing_chan: (Sender<T>, Receiver<T>), incoming_chan: (Sender<T>, Receiver<T>)) -> Result<Self, anyhow::Error> {
        let read_stream = connect_to(ip_addr)?;
        let write_stream = read_stream.try_clone()?;

        let mut chan = BiDirectionalTcpChannel {
            rx_chan: incoming_chan,
            tx_chan: outgoing_chan,
        };

        chan.poll(128, mio::net::TcpStream::from_std(read_stream), mio::net::TcpStream::from_std(write_stream))?;

        return Ok(chan)
    }

    pub fn channel(&self) -> (Sender<T>, Receiver<T>) {
        (self.tx_chan.0.clone(), self.rx_chan.1.clone())
    }

    fn tcp_channel(&self) -> (&Sender<T>, &Receiver<T>) {
        (&self.rx_chan.0, &self.tx_chan.1)
    }

    pub fn poll(&mut self, num_events: usize, rx_stream: mio::net::TcpStream, tx_stream: mio::net::TcpStream) -> Result<JoinHandle<Result<(), anyhow::Error>>, anyhow::Error> {
        let poller = Poll::new()?;
        let mut events = Events::with_capacity(num_events);

        const DATA_READ:  Token = Token(0);
        const DATA_WRITE: Token = Token(1);

        poller.registry().register(&mut rx_stream, DATA_READ, Interest::READABLE)?;
        poller.registry().register(&mut tx_stream, DATA_WRITE, Interest::WRITABLE)?;

        let handle = std::thread::spawn(move || {
            let rx_chan = &mut self.rx_chan.0.clone();
            let tx_chan = &mut self.tx_chan.1.clone();

            loop {
                poller.poll(&mut events, None)?;

                for event in events.iter() {
                    match event.token() {
                        DATA_READ => {
                            let data = bincode::deserialize_from(&rx_stream)?;
                            rx_chan.send(data)?;
                        },

                        DATA_WRITE => {
                            if let Ok(t) = tx_chan.recv() {
                                let data = bincode::serialize(&t)?;
                                tx_stream.write_all(&data)?
                            } else {
                                break;
                            }
                        },

                        token => {
                            println!("{:#?}", token);
                        }
                    }
                }
            }
        });

        return Ok(handle)
    }

    pub fn close(self) {
        let (left_read, right_read) = self.rx_chan;
        let (left_write, right_write) = self.tx_chan;

        drop(left_read);
        drop(right_read);
        drop(left_write);
        drop(right_write);
    }
}

pub fn connect_to(ip_addr: SocketAddr) -> Result<TcpStream, anyhow::Error> {
    Ok(TcpStream::connect(ip_addr)?)
}
