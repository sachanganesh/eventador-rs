use std::net::SocketAddr;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::net::TcpStream;
use std::io::prelude::*;
use std::thread::JoinHandle;

pub struct BiDirectionalTcpChannel<T> {
    incoming_chan: (Sender<T>, Receiver<T>),
    outgoing_chan: (Sender<T>, Receiver<T>),
    consumer_handle: JoinHandle<()>,
    producer_handle: JoinHandle<()>
}

impl<T: 'static + Send + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>> BiDirectionalTcpChannel<T> {
    pub fn new(ip_addr: SocketAddr) -> Result<Self, std::io::Error> {
        Self::from(ip_addr, unbounded(), unbounded())
    }

    pub fn from(ip_addr: SocketAddr, incoming_chan: (Sender<T>, Receiver<T>), outgoing_chan: (Sender<T>, Receiver<T>)) -> Result<Self, std::io::Error> {
        let read_stream = connect_to(ip_addr)?;
        let write_stream = read_stream.try_clone()?;

        let consumer_handle = spawn_consumer(incoming_chan.0.clone(), read_stream);
        let producer_handle = spawn_producer(outgoing_chan.1.clone(), write_stream);

        Ok(BiDirectionalTcpChannel {
            incoming_chan,
            outgoing_chan,
            consumer_handle,
            producer_handle
        })
    }

    pub fn channel(&self) -> (Sender<T>, Receiver<T>) {
        (self.outgoing_chan.0.clone(), self.incoming_chan.1.clone())
    }

    pub fn close(self) {
        drop(self)
    }
}

pub fn connect_to(ip_addr: SocketAddr) -> Result<TcpStream, std::io::Error> {
    Ok(TcpStream::connect(ip_addr)?)
}

pub fn spawn_producer<T: 'static + Send + serde::ser::Serialize>(chan: Receiver<T>, mut conn: TcpStream) -> JoinHandle<()> {
    std::thread::spawn(move || {
        use std::io::Write;

        for t in chan.iter() {
            let data = bincode::serialize(&t).ok().expect("serializable data to successfully serialize");
            conn.write_all(&data).ok();
        }
    })
}

pub fn spawn_consumer<T: 'static + Send + for<'de> serde::de::Deserialize<'de>>(chan: Sender<T>, conn: TcpStream) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            let data = bincode::deserialize_from(&conn).ok().unwrap();
            chan.send(data).unwrap()
        }
    })
}
