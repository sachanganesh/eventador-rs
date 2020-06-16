pub mod dist_chan;

use dist_chan::*;
use crossbeam_channel::{Receiver, Sender};

fn main() {
    let (outgoing_sender, outgoing_receiver): (Sender<String>, Receiver<String>) = crossbeam_channel::unbounded();
    let (incoming_sender, incoming_receiver): (Sender<String>, Receiver<String>) = crossbeam_channel::unbounded();

    let read_conn = connect_to("127.0.0.1:5678".parse().unwrap()).ok().unwrap();
    let write_conn = read_conn.try_clone().ok().unwrap();

    std::thread::spawn(move || {
        produce(outgoing_receiver.clone(), write_conn)
    });

    std::thread::spawn(move || {
        consume(incoming_sender.clone(), read_conn)
    });

    outgoing_sender.send(String::from("Hello, world!")).ok().unwrap();
    for data in incoming_receiver.iter() {
        println!("{}", data);
    }
}
