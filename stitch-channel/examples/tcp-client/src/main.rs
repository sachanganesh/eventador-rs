use log::*;
use std::env;
use stitch_channel::tcp::BidirectionalTcpAgent;
use stitch_channel::Sender;

#[async_std::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    // Get ip address from cmd line args
    let args: Vec<String> = env::args().collect();
    let ip_address = match args.get(1) {
        Some(addr) => addr,
        None => {
            error!("Need to pass IP address to bind to as command line argument");
            panic!();
        }
    };

    // create a client connection to the server
    let dist_chan =
        BidirectionalTcpAgent::unbounded(ip_address).expect("Construction of TCP client failed");
    let (sender, receiver) = dist_chan.channel();

    // ping the server by writing a message to it
    ping_server(sender).await?;

    // wait for the server to reply with an ack
    if let Ok(msg) = receiver.recv().await {
        info!("Received ack message: {}", msg);
    }

    Ok(())
}

async fn ping_server(sender: Sender<String>) -> Result<(), anyhow::Error> {
    let msg = String::from("Hello world");

    info!("Sending message: {}", msg);
    sender.send(msg).await?;

    Ok(())
}
