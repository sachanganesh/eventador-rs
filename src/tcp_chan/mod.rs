use std::io::prelude::*;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use std::thread::JoinHandle;

use async_std::task;
use crossbeam_channel::{Receiver, Sender, unbounded, bounded};
use mio::{Events, Interest, Poll, Token};
use mio::net::TcpStream as MioTcpStream;

pub struct BiDirectionalTcpChannel<T> {
    rx_chan: (Sender<T>, Receiver<T>),
    tx_chan: (Sender<T>, Receiver<T>),
}

impl<T> BiDirectionalTcpChannel<T>
where T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de> {
    pub fn unbounded(ip_addr: SocketAddr) -> Result<Self, anyhow::Error> {
        Self::from(ip_addr, unbounded(), unbounded())
    }

    pub fn bounded(ip_addr: SocketAddr, outgoing_bound: Option<usize>, incoming_bound: Option<usize>) -> Result<Self, anyhow::Error> {
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
        let read_stream = Self::connect_to(ip_addr)?;
        read_stream.set_read_timeout(Some(Duration::from_secs(1)));

        let write_stream = read_stream.try_clone()?;
        write_stream.set_write_timeout(Some(Duration::from_secs(1)));

        let _receiver = incoming_chan.0.clone();
        let _sender   = outgoing_chan.1.clone();

        task::spawn(Self::poll(
            1024,
            Arc::new(MioTcpStream::from_std(read_stream)),
            Arc::new(MioTcpStream::from_std(write_stream)),
            _receiver,
            _sender
        ));

        Ok(BiDirectionalTcpChannel {
            rx_chan: incoming_chan,
            tx_chan: outgoing_chan,
        })
    }

    pub fn channel(&self) -> (Sender<T>, Receiver<T>) {
        (self.tx_chan.0.clone(), self.rx_chan.1.clone())
    }

    fn tcp_channel(&self) -> (&Sender<T>, &Receiver<T>) {
        (&self.rx_chan.0, &self.tx_chan.1)
    }

    pub fn close(self) {
        let (left_read, right_read) = self.rx_chan;
        let (left_write, right_write) = self.tx_chan;

        drop(left_read);
        drop(right_read);
        drop(left_write);
        drop(right_write);
    }

    pub fn connect_to(ip_addr: SocketAddr) -> Result<TcpStream, anyhow::Error> {
        Ok(TcpStream::connect(ip_addr)?)
    }

    pub async fn poll(
        num_events: usize,
        mut rx_stream: Arc<MioTcpStream>,
        mut tx_stream: Arc<MioTcpStream>,
        rx_chan: Sender<T>,
        tx_chan: Receiver<T>
    ) -> Result<(), anyhow::Error> {
        let mut poller = Poll::new().ok().unwrap();
        let mut events = Events::with_capacity(num_events);

        const DATA_READ: Token = Token(0);
        const DATA_WRITE: Token = Token(1);

        poller.registry().register(Arc::get_mut(&mut rx_stream).unwrap(), DATA_READ, Interest::READABLE).unwrap();
        poller.registry().register(Arc::get_mut(&mut tx_stream).unwrap(), DATA_WRITE, Interest::WRITABLE).unwrap();

        loop {
            poller.poll(&mut events, None)?;

            for event in events.iter() {
                match event.token() {
                    DATA_READ => {
                        let rx_stream = rx_stream.clone();
                        let rx_chan = rx_chan.clone();

                        task::spawn(async move {
                            Self::read_from_stream(&rx_stream, rx_chan)
                        });
                    },

                    DATA_WRITE => {
                        let tx_chan = tx_chan.clone();
                        let tx_stream = tx_stream.clone();

                        task::spawn(async move {
                            Self::write_to_stream(tx_chan, &tx_stream)
                        });
                    },

                    token => {
                        println!("unhandled: {:#?}", token);
                    }
                }
            }

            task::yield_now().await;
        }
    }

    fn read_from_stream(input: &MioTcpStream, mut output: Sender<T>) {
        const MAX_REDUCTIONS: usize = 200;
        let mut reductions = 0;

        while let Ok(data) = bincode::deserialize_from(input) {
            if let Ok(_) = output.try_send(data) {
                reductions += 1;
            } else {
                break;
            }

            if reductions == MAX_REDUCTIONS { break }
        }
    }

    fn write_to_stream(input: Receiver<T>, mut output: &MioTcpStream) {
        const MAX_REDUCTIONS: usize = 200;
        let mut reductions = 0;

        while let Ok(t) = input.try_recv() {
            if let Ok(data) = bincode::serialize(&t) {
                if let Ok(_) = output.write_all(&data) {
                    reductions += 1;
                }
            }

            if reductions == MAX_REDUCTIONS { break }
        }
    }
}