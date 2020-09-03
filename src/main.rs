use async_channel::{Receiver, Sender};
use async_std::{io, task};
use std::{fs::File, io::BufReader};
use uuid::Uuid;

use async_std::sync::Arc;
use seam_channel::net::tls::rustls::internal::pemfile::{certs, rsa_private_keys};
use seam_channel::net::tls::rustls::{ClientConfig, NoClientAuth, ServerConfig};
use seam_channel::net::*;

#[macro_use]
extern crate log;

const MAX_MESSAGES: usize = 150;
const DOMAIN: &str = "localhost";
const IP_ADDR: &str = "localhost:5678";

#[async_std::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    // let dist_chan = test_tcp()?;
    let dist_chan = test_tls()?;

    let (sender, receiver) = dist_chan.unbounded();

    let read_task = task::spawn(async_read(receiver));
    let _write_task = task::spawn(async_write(sender));

    dist_chan.ready()?;
    read_task.await;

    Ok(())
}

#[allow(dead_code)]
fn test_tcp() -> Result<StitchNetClient, anyhow::Error> {
    let (_server, conns) = StitchNetServer::tcp_server(IP_ADDR).expect("server doesn't work");
    let _handle = task::spawn(echo_server(conns));

    Ok(StitchNetClient::tcp_client(IP_ADDR)?)
}

#[allow(dead_code)]
fn test_tls() -> Result<StitchNetClient, anyhow::Error> {
    let mut config = ServerConfig::new(NoClientAuth::new());
    let cert_path = "/home/svganesh/Documents/tools/echo-server/async-tls/tests/end.cert";
    let key_path = "/home/svganesh/Documents/tools/echo-server/async-tls/tests/end.rsa";
    let certs = certs(&mut BufReader::new(File::open(cert_path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))?;
    let mut keys = rsa_private_keys(&mut BufReader::new(File::open(key_path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))?;
    config
        .set_single_cert(certs, keys.remove(0))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let acceptor = config.into();
    let (_, conns) = StitchNetServer::tls_server(IP_ADDR, acceptor)?;
    let _handle = task::spawn(echo_server(conns));

    let mut client_config = ClientConfig::new();
    let client_file =
        std::fs::read("/home/svganesh/Documents/tools/echo-server/async-tls/tests/end.chain")
            .unwrap();
    let mut client_pem = std::io::Cursor::new(client_file);
    client_config
        .root_store
        .add_pem_file(&mut client_pem)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid cert"))
        .expect("it works");
    match StitchNetClient::tls_client(IP_ADDR, DOMAIN, client_config.into()) {
        Ok(data) => Ok(data),
        Err(err) => {
            error!("{}", err);
            panic!(err);
        }
    }
}

async fn async_read(receiver: Receiver<String>) {
    for i in 0..MAX_MESSAGES + 1 {
        if let Ok(data) = receiver.recv().await {
            info!("Received #{}: {}", i, data);
        }
    }
}

async fn async_write(sender: Sender<String>) -> Result<(), anyhow::Error> {
    for i in 0..MAX_MESSAGES {
        let id = Uuid::new_v4();
        let msg = format!("Hello, {}", id);

        info!("Sending #{}: {}", i, msg);
        sender.send(msg).await?;
    }

    Ok(())
}

async fn echo_server(connections: Receiver<Arc<StitchNetClient>>) {
    for conn in connections.recv().await {
        task::spawn(async move {
            let (sender, receiver) = conn.unbounded::<String>();

            conn.ready().expect("could not start read and write tasks");

            while let Ok(msg) = receiver.recv().await {
                info!("Echoing message: {}", msg);

                if let Err(err) = sender.send(msg).await {
                    error!("Could not echo message: {:#?}", err);
                }
            }
        });
    }
}
