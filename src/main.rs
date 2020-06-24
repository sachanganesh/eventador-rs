pub mod dist_chan;

use std::iter;

use async_std::net::ToSocketAddrs;
use async_std::task;
use rand::{Rng, distributions::Alphanumeric, thread_rng};

fn main() -> Result<(), anyhow::Error> {
    let ip = task::block_on("127.0.0.1:5678".to_socket_addrs())?.next().unwrap();

    let dist_chan = dist_chan::tcp::bidi_chan::BiDirectionalTcpChannel::unbounded(ip).unwrap();
    let (sender, receiver) = dist_chan.channel();

    // sender.send(String::from("Hello, world!")).unwrap();
    // sender.send(String::from("Hello, birds!")).unwrap();
    // sender.send(String::from("Hello, trees!")).unwrap();
    // sender.send(String::from("Hello, flowers!")).unwrap();
    // sender.send(String::from("Hello, you!")).unwrap();

    // let mut rng = thread_rng();
    for i in 0..5 {
        // let sample: String = iter::repeat(())
        //     .map(|()| rng.sample(Alphanumeric))
        //     .take(7)
        //     .collect();
        // sender.send(sample).expect("todo");

        sender.send(i).expect("todo");
    }

    for _ in 0..5 {
        let data = receiver.recv().ok().unwrap();
        println!("{}", data);
    }

    dist_chan.close();

    Ok(())
}
