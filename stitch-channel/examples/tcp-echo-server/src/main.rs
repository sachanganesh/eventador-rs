use async_std::task;
use log::*;
use std::env;
use stitch_channel::net::{StitchClient, StitchNetClient, StitchNetServer};
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
    let (_server, conns) = StitchNetServer::tcp_server(ip_address)?;

    // handle server connections
    // wait for a connection to come in and be accepted
    while let Ok(conn) = conns.recv().await {
        info!("Handling connection from {}", conn.peer_addr());

        // register for String-typed messages
        let (sender, receiver) = conn.unbounded::<String>();

        // handle String messages
        let _handle = task::spawn(async move {
            // for every String message
            while let Ok(msg) = receiver.recv().await {
                info!("Echoing message: {}", msg);

                let response = format!("{}, right back at you!", msg);

                // Send the message back to its source
                if let Err(err) = sender.send(response).await {
                    error!("Could not echo message: {:#?}", err);
                }
            }
        });

        // let the connection know you are ready to send and receive messages
        conn.ready().expect("could not ready the connection for reading and writing");
    }

    Ok(())
}
