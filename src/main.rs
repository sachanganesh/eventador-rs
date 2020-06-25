pub mod dist_chan;

use std::io::Write;
use std::time::SystemTime;

use async_std::net::ToSocketAddrs;
use async_std::task;
use crossbeam_channel::{Receiver, Sender, unbounded, bounded};

use dist_chan::tcp::bidi_chan::{MAX_MESSAGES, BiDirectionalTcpChannel};

fn main() -> Result<(), anyhow::Error> {
    let now = SystemTime::now();

    let ip = task::block_on("127.0.0.1:5678".to_socket_addrs())?.next().unwrap();

    let dist_chan: BiDirectionalTcpChannel<Vec<u8>> = BiDirectionalTcpChannel::bounded(ip, Some(1), Some(1)).unwrap();
    let (sender, receiver) = dist_chan.channel();

    let read_task = task::spawn(async_read(receiver));

    let write_task = task::spawn(async_write(sender));

    // dist_chan.close();

    let R = task::block_on(read_task);
    let W = task::block_on(write_task);
    let r = task::block_on(dist_chan.reader);
    let w = task::block_on(dist_chan.writer);


    let mut R_transform: Vec<String> = R.into_iter().map(|time| {
        format!("{:0>16}R", time.duration_since(now).unwrap().as_micros().to_string())
    }).collect();

    let mut W_transform: Vec<String> = W.into_iter().map(|time| {
        format!("{:0>16}W", time.duration_since(now).unwrap().as_micros().to_string())
    }).collect();

    let mut r_transform: Vec<String> = r.into_iter().map(|time| {
        format!("{:0>16}r", time.duration_since(now).unwrap().as_micros().to_string())
    }).collect();

    let mut w_transform: Vec<String> = w.into_iter().map(|time| {
        format!("{:0>16}w", time.duration_since(now).unwrap().as_micros().to_string())
    }).collect();


    let mut arr = R_transform;
    arr.append(&mut W_transform);
    arr.append(&mut r_transform);
    arr.append(&mut w_transform);

    arr.sort();

    let order: Vec<String> = arr.into_iter().map(|mut elem| {
        elem.pop().unwrap().to_string()
    }).collect();

    println!("{:?}", order.join(""));

    Ok(())
}

async fn async_read(receiver: Receiver<Vec<u8>>) -> Vec<SystemTime> {
    let mut cnt = 0;

    let mut arr: Vec<SystemTime> = Vec::with_capacity(MAX_MESSAGES);
    let mut i = 0;

    loop {
        let data = receiver.recv().ok().unwrap();

        cnt += 1;

        // print!("R");
        // std::io::stdout().flush();

        arr.push(SystemTime::now());
        i += 1;

        if i == MAX_MESSAGES {
            break;
        }
    }

    return arr;
}

async fn async_write(sender: Sender<Vec<u8>>) -> Vec<SystemTime> {
    let buf: Vec<u8> = vec![0; 100];

    let mut arr: Vec<SystemTime> = Vec::with_capacity(MAX_MESSAGES);
    let mut i = 0;

    for _ in 0..100 {
        arr.push(SystemTime::now());
        sender.send(buf.clone());

        // print!("W");
        // std::io::stdout().flush();

        i += 1;

        if i == MAX_MESSAGES {
            break;
        }

        // sender.send(String::from("Hello, world!")).unwrap();
        // println!("sent message to channel");
        // sender.send(String::from("Hello, birds!")).unwrap();
        // println!("sent message to channel");
        // sender.send(String::from("Hello, trees!")).unwrap();
        // println!("sent message to channel");
        // sender.send(String::from("Hello, flowers!")).unwrap();
        // println!("sent message to channel");
        // sender.send(String::from("Hello, you!")).unwrap();
        // println!("sent message to channel");
    }

    return arr;
}
