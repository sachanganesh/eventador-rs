use async_channel::{Receiver, Sender};
use async_std::{io, task};
use std::{fs::File, io::BufReader};
use uuid::Uuid;

use stitch_channel::net::tcp::BidirectionalTcpAgent as TcpChannel;
// use stitch_channel::net::tls::rustls::internal::pemfile::{certs, rsa_private_keys};
// use stitch_channel::net::tls::{
//     rustls::{ClientConfig, NoClientAuth, ServerConfig},
//     BiDirectionalTlsChannel as TlsChannel, TlsServer,
// };

#[macro_use]
extern crate log;

const MAX_MESSAGES: usize = 1;
const DOMAIN: &str = "localhost";
const IP_ADDR: &str = "localhost:5678";

#[async_std::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let mut dist_chan = test_tcp()?;
    // let dist_chan = test_tls()?;

    let (sender, receiver) = dist_chan.unbounded();

    let read_task = task::spawn(async_read(receiver));
    let _write_task = task::spawn(async_write(sender));

    read_task.await;

    Ok(())
}

// async fn handle_connections(
//     (sender, receiver): (Sender<dyn StitchMessage>, Receiver<dyn StitchMessage>),
// ) {
//     debug!("Starting echo loop");
//
//     while let Ok(data) = receiver.recv().await {
//         // info!("Echoing: {}", data);
//         sender.send(data).await.expect("it works");
//     }
// }

fn test_tcp() -> Result<TcpChannel, anyhow::Error> {
    // let echo_server = TcpServer::unbounded(IP_ADDR, handle_connections).expect("it works");
    Ok(TcpChannel::new(IP_ADDR).expect("it works"))
}

async fn async_read(receiver: Receiver<Box<String>>) {
    for i in 0..MAX_MESSAGES + 1 {
        if let Ok(data) = receiver.recv().await {
            info!("Received #{}: {}", i, data);
        }
    }
}

async fn async_write(sender: Sender<Box<String>>) -> Result<(), anyhow::Error> {
    for i in 0..MAX_MESSAGES {
        let id = Uuid::new_v4();
        let msg = format!("Hello, {}", id);

        info!("Sending #{}: {}", i, msg);
        sender.send(Box::new(msg)).await?;
    }

    Ok(())
}

// fn test_tls() -> Result<TlsChannel<String>, anyhow::Error> {
//     let mut config = ServerConfig::new(NoClientAuth::new());
//     let cert_path = "/home/svganesh/Documents/tools/echo-server/async-tls/tests/end.cert";
//     let key_path = "/home/svganesh/Documents/tools/echo-server/async-tls/tests/end.rsa";
//     let certs = certs(&mut BufReader::new(File::open(cert_path)?))
//         .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))?;
//     let mut keys = rsa_private_keys(&mut BufReader::new(File::open(key_path)?))
//         .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))?;
//     config
//         .set_single_cert(certs, keys.remove(0))
//         .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
//     let acceptor = config.into();
//     let echo_server =
//         TlsServer::unbounded(IP_ADDR, acceptor, handle_connections).expect("it works");
//
//     let mut client_config = ClientConfig::new();
//     let client_file =
//         std::fs::read("/home/svganesh/Documents/tools/echo-server/async-tls/tests/end.chain")
//             .unwrap();
//     let mut client_pem = std::io::Cursor::new(client_file);
//     client_config
//         .root_store
//         .add_pem_file(&mut client_pem)
//         .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid cert"))
//         .expect("it works");
//     match TlsChannel::unbounded(IP_ADDR, DOMAIN, client_config.into()) {
//         Ok(data) => Ok(data),
//         Err(err) => {
//             error!("{}", err);
//             panic!(err);
//         }
//     }
// }
