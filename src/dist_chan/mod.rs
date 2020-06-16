use std::net::SocketAddr;
use crossbeam_channel::{Receiver, Sender};
use std::net::TcpStream;
use std::io::prelude::*;

pub fn connect_to(ip_addr: SocketAddr) -> Result<TcpStream, std::io::Error> {
    Ok(TcpStream::connect(ip_addr)?)
}

pub fn produce<T: serde::ser::Serialize>(chan: Receiver<T>, mut conn: TcpStream) {
    use std::io::Write;

    for t in chan.iter() {
        let data = bincode::serialize(&t).ok().expect("serializable data to successfully serialize");
        conn.write_all(&data).ok();
    }
}

pub fn consume<T: for<'de> serde::de::Deserialize<'de>>(chan: Sender<T>, conn: TcpStream) {
    let data = bincode::deserialize_from(conn).ok().unwrap();
    chan.send(data).ok().unwrap()
}
