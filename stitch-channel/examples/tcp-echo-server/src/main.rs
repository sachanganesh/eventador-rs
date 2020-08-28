use async_std::task;
use log::*;
use std::env;
use stitch_channel::net::tcp::{TcpClientAgent, TcpServerAgent};
use stitch_channel::{Arc, Receiver};

#[async_std::main]
async fn main() -> anyhow::Result<()> {
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

    // create a server
    let (_server, conns) = TcpServerAgent::new(ip_address)?;

    // handle server connections
    Ok(echo_server(conns).await)
}

async fn echo_server(connections: Receiver<Arc<TcpClientAgent>>) {
    // wait for a connection to come in and be accepted
    for conn in connections.recv().await {
        // register for String-typed messages
        let (sender, receiver) = conn.unbounded::<String>();

        // handle String messages
        task::spawn(async move {
            // for every String message
            while let Ok(msg) = receiver.recv().await {
                info!("Echoing message: {}", msg);

                let response = format!("{}, right back at you!", msg);

                // Send the message back to its source
                if let Err(err) = sender.send(response).await {
                    error!("Could not echo message: {:#?}", err);
                }
            }
        }).await;
    }
}
