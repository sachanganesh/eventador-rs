pub mod tcp_chan;

use tcp_chan::*;

fn main() {
    let ip = "127.0.0.1:5678".parse().unwrap();
    let dist_chan = BiDirectionalTcpChannel::unbounded(ip).unwrap();
    let (sender, receiver) = dist_chan.channel();

    sender.send(String::from("Hello, world!")).unwrap();
    sender.send(String::from("Hello, birds!")).unwrap();
    sender.send(String::from("Hello, trees!")).unwrap();
    sender.send(String::from("Hello, flowers!")).unwrap();
    sender.send(String::from("Hello, you!")).unwrap();

    for _ in 0..5 {
        let data = receiver.recv().ok().unwrap();
        println!("{}", data);
    }
}
