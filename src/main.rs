pub mod dist_chan;

use async_std::net::ToSocketAddrs;
use async_std::task;
use crossbeam_channel::{Receiver, Sender};
use uuid::Uuid;

use dist_chan::tcp::bidi_chan::BiDirectionalTcpChannel;

const MAX_MESSAGES: usize = 150;

fn main() -> Result<(), anyhow::Error> {
    let ip = task::block_on("127.0.0.1:5678".to_socket_addrs())?.next().unwrap();

    let dist_chan = BiDirectionalTcpChannel::bounded(ip, Some(1), Some(1)).unwrap();
    let (sender, receiver) = dist_chan.channel();

    let read_task   = task::spawn(async_read(receiver));
    let _write_task = task::spawn(async_write(sender));

    task::block_on(read_task);

    Ok(())
}

async fn async_read(receiver: Receiver<String>) {
    for i in 0..MAX_MESSAGES {
        if let Ok(data) = receiver.recv() {
            println!("received #{}: {}", i, data);
        }
    }
}

async fn async_write(sender: Sender<String>) -> Result<(), anyhow::Error> {
    for _ in 0..MAX_MESSAGES {
        let id = Uuid::new_v4();
        sender.send(format!("Hello, {}", id))?;
    }

    Ok(())
}
