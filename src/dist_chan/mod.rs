use std::net::SocketAddr;
use crossbeam_channel::{Receiver, Sender};
use mio::{Interest, Poll, Token, net::TcpStream};

pub struct BiDirectionalChannel<T: serde::Serialize + Send> {
    conn: TcpStream,
    incoming: Receiver<T>,
    outgoing: Sender<T>,
}

impl<T: 'static + serde::Serialize + Send> BiDirectionalChannel<T> {
    pub fn from_channel(ip_addr: SocketAddr, (send, recv): (Sender<T>, Receiver<T>)) -> Result<Self, std::io::Error> {
        let poll = Poll::new()?;
        let listener: Token = Token(ip_addr.port() as usize);

        let mut stream = Self::connect_to(ip_addr)?;
        poll.registry().register(&mut stream, listener, Interest::READABLE | Interest::WRITABLE)?;


        // let write_ref = std::thread::spawn(move || { Self::produce(recv, &mut stream) });


        Ok(BiDirectionalChannel {
            conn: stream,
            incoming: recv,
            outgoing: send
        })
    }

    fn connect_to(ip_addr: SocketAddr) -> Result<TcpStream, std::io::Error> {
        TcpStream::connect(ip_addr)
    }

    fn produce(chan: Receiver<T>, conn: &mut TcpStream) {
        use std::io::Write;

        for t in chan.iter() {
            let data = bincode::serialize(&t).ok().expect("serializable data to successfully serialize");
            conn.write(&data).ok();
        }
    }
}
