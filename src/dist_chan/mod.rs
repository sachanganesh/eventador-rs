use std::net::SocketAddr;
use crossbeam_channel::{Receiver, Sender, unbounded, bounded};
use std::net::TcpStream;
use std::io::prelude::*;
use std::thread::JoinHandle;

pub struct BiDirectionalTcpChannel<T> {
    rx_chan: (Sender<T>, Receiver<T>),
    tx_chan: (Sender<T>, Receiver<T>),
    thread_handles: (JoinHandle<()>, JoinHandle<()>)
}

impl<T: 'static + Send + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>> BiDirectionalTcpChannel<T> {
    pub fn new(ip_addr: SocketAddr) -> Result<Self, std::io::Error> {
        Self::from(ip_addr, unbounded(), unbounded())
    }

    pub fn new_bounded(ip_addr: SocketAddr, outgoing_bound: Option<usize>, incoming_bound: Option<usize>) -> Result<Self, std::io::Error> {
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

    pub fn from(ip_addr: SocketAddr, outgoing_chan: (Sender<T>, Receiver<T>), incoming_chan: (Sender<T>, Receiver<T>)) -> Result<Self, std::io::Error> {
        let read_stream = connect_to(ip_addr)?;
        let write_stream = read_stream.try_clone()?;

        let consumer_handle = spawn_consumer(incoming_chan.0.clone(), read_stream);
        let producer_handle = spawn_producer(outgoing_chan.1.clone(), write_stream);

        Ok(BiDirectionalTcpChannel {
            rx_chan: incoming_chan,
            tx_chan: outgoing_chan,
            thread_handles: (consumer_handle, producer_handle)
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

        let (consumer, producer) = self.thread_handles;

        consumer.join().unwrap();
        producer.join().unwrap();
    }
}

pub fn connect_to(ip_addr: SocketAddr) -> Result<TcpStream, std::io::Error> {
    Ok(TcpStream::connect(ip_addr)?)
}

pub fn spawn_producer<T: 'static + Send + serde::ser::Serialize>(chan: Receiver<T>, mut conn: TcpStream) -> JoinHandle<()> {
    std::thread::spawn(move || {
        use std::io::Write;

        while let Ok(t) = chan.recv() {
            let data = bincode::serialize(&t).ok().expect("serializable data to successfully serialize");
            conn.write_all(&data).ok();
        }
    })
}

pub fn spawn_consumer<T: 'static + Send + for<'de> serde::de::Deserialize<'de>>(chan: Sender<T>, conn: TcpStream) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            let data = bincode::deserialize_from(&conn).ok().unwrap();

            if chan.send(data).is_err() { break }
        }
    })
}
