use std::env;
use log::error;
use stitch_channel::tcp::server::TcpServer;
use stitch_channel::{Sender, Receiver};

fn main() -> Result<(), anyhow::Error> {
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

    let _echo_server = TcpServer::unbounded(ip_address, handle_connections)?;

    loop {}
}

async fn handle_connections((sender, receiver): (Sender<String>, Receiver<String>)) {
    while let Ok(msg) = receiver.recv().await {
        let modified_msg = String::from(msg + ", right back at you!");
        sender.send(modified_msg).await.expect("Message sending failed");
    }
}
