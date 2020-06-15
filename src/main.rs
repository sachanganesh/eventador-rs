pub mod dist_chan;

use dist_chan::*;

fn main() {
    let d_chan: BiDirectionalChannel<String> = BiDirectionalChannel::from_channel("127.0.0.1:5678".parse().unwrap(), crossbeam_channel::unbounded()).ok().unwrap();

    println!("Hello, world!");
}
