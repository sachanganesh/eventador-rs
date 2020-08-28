use log::*;
use std::env;
use stitch_channel::net::tcp::TcpClientAgent;
use stitch_channel::Sender;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Get ip address from cmd line args
    let args: Vec<String> = env::args().collect();
    let ip_address = match args.get(1) {
        Some(addr) => addr,
        None => {
            error!("Need to pass IP address to connect to as command line argument");
            panic!();
        }
    };

    // create a client connection to the server
    let dist_chan = TcpClientAgent::new(ip_address)?;

    // create a channel for String messages on the TCP connection
    let (sender, receiver) = dist_chan.bounded::<String>(Some(100));

    // ping the server by sending a message
    ping_server(sender).await?;

    // wait for the server to reply with an ack
    if let Ok(msg) = receiver.recv().await {
        info!("Received ack message: {}", msg);
    }

    Ok(())
}

async fn ping_server(sender: Sender<String>) -> anyhow::Result<()> {
    let msg = String::from("Hello world");

    info!("Sending message: {}", msg);
    sender.send(msg).await?;

    Ok(())
}
