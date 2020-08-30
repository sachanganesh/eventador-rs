use async_std::{io, task};
use log::*;
use seam_channel::net::tls::rustls::internal::pemfile::{certs, rsa_private_keys};
use seam_channel::net::tls::rustls::{NoClientAuth, ServerConfig};
use seam_channel::net::{StitchClient, StitchNetServer};
use std::env;
use std::fs::File;
use std::io::BufReader;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Get ip address from cmd line args
    let (ip_address, cert_path, key_path) = parse_args();

    let certs = certs(&mut BufReader::new(File::open(cert_path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))?;

    let mut keys = rsa_private_keys(&mut BufReader::new(File::open(key_path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))?;

    let mut config = ServerConfig::new(NoClientAuth::new());
    config
        .set_single_cert(certs, keys.remove(0))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

    // create a server
    let (_server, conns) = StitchNetServer::tls_server(ip_address, config.into())?;

    // handle server connections
    // wait for a connection to come in and be accepted
    while let Ok(conn) = conns.recv().await {
        info!("Handling connection: {}", conn.peer_addr());

        // register for String-typed messages
        let (sender, receiver) = conn.unbounded::<String>();

        // let the connection know you are ready to send and receive messages
        conn.ready()
            .expect("could not ready the connection for reading and writing");

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
        });
    }

    Ok(())
}

fn parse_args() -> (String, String, String) {
    let args: Vec<String> = env::args().collect();

    let ip_address = match args.get(1) {
        Some(addr) => addr,
        None => {
            error!("Need to pass IP address to connect to as first command line argument");
            panic!();
        }
    };

    let cert_path = match args.get(2) {
        Some(d) => d,
        None => {
            error!("Need to pass path to cert file as second command line argument");
            panic!();
        }
    };

    let key_path = match args.get(3) {
        Some(d) => d,
        None => {
            error!("Need to pass path to key file as third command line argument");
            panic!();
        }
    };

    (
        ip_address.to_string(),
        cert_path.to_string(),
        key_path.to_string(),
    )
}
