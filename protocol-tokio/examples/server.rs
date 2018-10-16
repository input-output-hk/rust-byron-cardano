extern crate tokio;
extern crate log;
extern crate env_logger;

use protocol_tokio::{Connection, AcceptingError};

use tokio::net::{TcpListener};
use tokio::prelude::*;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Parse the address of whatever address we're listening to
    let addr = "127.0.0.1:3000".parse().unwrap();

    let server = TcpListener::bind(&addr).unwrap().incoming().into_future()
    .map_err(|(err, _)| AcceptingError::IoError(err))
    .and_then(|(stream, _)| {
        let stream = stream.unwrap();
        println!("accept stream: {}", stream.peer_addr().unwrap());

        Connection::accept(stream)
    }).map_err(|err| {
        println!("accepting error = {:?}", err);
    }).map(|_| {
        println!("Accepting succeed");
    });

    println!("About to create the server and wait for connection...");
    tokio::run(server);
    println!("Server has run, received a connection and stopped");
}
