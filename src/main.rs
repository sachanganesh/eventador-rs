pub mod tcp_chan;

use async_std::net::ToSocketAddrs;
use async_std::task;

use tcp_chan::*;

fn main() -> Result<(), anyhow::Error> {
    let ip = task::block_on("127.0.0.1:5678".to_socket_addrs())?.next().unwrap();

    let dist_chan = bidi_chan::BiDirectionalTcpChannel::unbounded(ip).unwrap();
    let (sender, receiver) = dist_chan.channel();

    sender.send(String::from("Hello, world!")).unwrap();
    sender.send(String::from("Hello, birds!")).unwrap();
    sender.send(String::from("Hello, trees!")).unwrap();
    sender.send(String::from("Hello, flowers!")).unwrap();
    sender.send(String::from("Hello, you!")).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("receiving!");
    for _ in 0..5 {
        let data = receiver.recv().ok().unwrap();
        println!("{}", data);
    }

    dist_chan.close();

    Ok(())
}
