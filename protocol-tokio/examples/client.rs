extern crate tokio;
extern crate log;
extern crate env_logger;
extern crate protocol_tokio;

use protocol_tokio::{Connection, ConnectingError};

use tokio::net::TcpStream;
use tokio::{prelude::{*}};

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // let addr = "13.229.185.80:3000".parse().unwrap();
    let addr = "127.0.0.1:3000".parse().unwrap();

    let client = TcpStream::connect(&addr)
    .map_err(ConnectingError::IoError)
    .and_then(|stream| {
        println!("created stream");

        Connection::connect(stream)
    }).map_err(|err| {
        println!("connection error = {:?}", err);
    }).map(|_| {
        println!("Connection succeed");
    });

    println!("About to create the stream and write to it...");
    tokio::run(client);
    println!("Stream has been created and written to.");
}
