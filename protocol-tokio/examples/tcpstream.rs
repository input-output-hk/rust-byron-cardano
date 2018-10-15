extern crate tokio;
extern crate log;
extern crate env_logger;

use protocol_tokio::network_transport::*;

use tokio::net::TcpStream;
use tokio::prelude::*;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Parse the address of whatever server we're talking to
    let addr = "13.229.185.80:3000".parse().unwrap();

    let client = TcpStream::connect(&addr)
    .map_err(ConnectingError::IoError)
    .and_then(|stream| {
        println!("created stream");

        Connection::connect(stream)
    }).map_err(|err| {
        // All tasks must have an `Error` type of `()`. This forces error
        // handling and helps avoid silencing failures.
        //
        // In our example, we are only going to log the error to STDOUT.
        println!("connection error = {:?}", err);
    }).map(|_| {
        println!("Connection succeed");
    });

    println!("About to create the stream and write to it...");
    tokio::run(client);
    println!("Stream has been created and written to.");
}
