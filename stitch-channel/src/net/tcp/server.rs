use crate::net::tcp::BidirectionalTcpAgent;

use async_channel::{bounded, unbounded, Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::prelude::*;
use async_std::task;
use log::*;

pub struct TcpServer {
    accept_loop_task: task::JoinHandle<Result<()>>,
}

impl TcpServer {
    pub fn unbounded<
        A: ToSocketAddrs + std::fmt::Display,
        T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
        F: 'static + Send + async_std::future::Future,
        H: 'static + Send + FnMut((Sender<T>, Receiver<T>)) -> F,
    >(
        ip_addrs: A,
        mut connection_handler: H,
    ) -> Result<Self>
    where
        <F as std::future::Future>::Output: std::marker::Send,
    {
        info!("Starting TCP server at {}", ip_addrs);
        let listener = task::block_on(TcpListener::bind(ip_addrs))?;

        let handler = task::spawn(async move {
            let mut incoming_stream = listener.incoming();

            // @todo implement recovery/retry mechanism
            loop {
                trace!("Reading from the stream of incoming connections");
                match incoming_stream.next().await {
                    Some(Ok(write_stream)) => {
                        let addr = write_stream.peer_addr()?;
                        let read_stream = write_stream.clone();
                        match BidirectionalTcpAgent::from_raw_parts(
                            (read_stream, write_stream),
                            unbounded(),
                            unbounded(),
                        ) {
                            Ok(dist_chan) => {
                                info!(
                                    "Accepted a connection from {} and passing to handler fn",
                                    addr
                                );
                                task::spawn(connection_handler(dist_chan.channel()));
                            }

                            Err(err) => {
                                error!("Encountered error when creating TCP channel: {:#?}", err)
                            }
                        }
                    }

                    Some(Err(err)) => error!(
                        "Encountered error when accepting TCP connection: {:#?}",
                        err
                    ),
                    None => unreachable!(),
                }
            }
        });

        Ok(TcpServer {
            accept_loop_task: handler,
        })
    }

    pub fn accept_loop_task(&self) -> &task::JoinHandle<Result<()>> {
        &self.accept_loop_task
    }

    pub fn close(self) {
        task::block_on(self.accept_loop_task.cancel());
    }
}
