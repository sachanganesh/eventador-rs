pub mod dist_chan;

use dist_chan::*;
use crossbeam_channel::{Receiver, Sender};

fn main() {
    let dist_chan = BiDirectionalTcpChannel::new("127.0.0.1:5678".parse().unwrap()).unwrap();
    let (sender, receiver) = dist_chan.channel();

    sender.send(String::from("Hello, world!")).unwrap();
    sender.send(String::from("Hello, birds!")).unwrap();
    sender.send(String::from("Hello, trees!")).unwrap();
    sender.send(String::from("Hello, flowers!")).unwrap();
    sender.send(String::from("Hello, you!")).unwrap();

    for data in receiver {
        println!("{}", data);
    }
}
