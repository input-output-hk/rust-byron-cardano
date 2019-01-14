extern crate env_logger;
extern crate futures;
extern crate log;
extern crate protocol_tokio;
extern crate tokio;

use protocol_tokio::{ConnectingError, Connection, InboundStream, Message, OutboundSink};

use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::timer::Delay;

use futures::future;

use std::time::{Duration, Instant};

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // let addr = "13.229.185.80:3000".parse().unwrap();
    let addr = "127.0.0.1:3000".parse().unwrap();

    let client = TcpStream::connect(&addr)
        .map_err(ConnectingError::IoError)
        .and_then(|stream| {
            println!("created stream");

            Connection::connect(stream)
        })
        .and_then(|connection| {
            let (sink, stream): (
                OutboundSink<_, u8, u8, u8, u8>,
                InboundStream<_, u8, u8, u8, u8>,
            ) = connection.split();

            let stream = stream
                .for_each(move |inbound| {
                    println!("inbound: {:?}", inbound);
                    future::empty()
                })
                .map_err(|err| println!("connection stream error {:#?}", err));

            let when = Instant::now() + Duration::from_millis(100);
            let sink = Delay::new(when)
                .then(|_| sink.new_light_connection())
                .and_then(|(lwcid, sink)| {
                    let when = Instant::now() + Duration::from_millis(100);
                    Delay::new(when)
                        .then(move |_| sink.send(Message::Bytes(lwcid, "Hello world".into())))
                        .and_then(move |sink| sink.close_light_connection(lwcid))
                })
                .and_then(|mut sink| sink.close())
                .map_err(|err| println!("err {:?}", err))
                .map(|_| ());

            stream.select(sink).then(|_| {
                println!("closing connection");
                Ok(())
            })
        })
        .map_err(|err| {
            println!("connection error = {:?}", err);
        })
        .map(|_| {
            println!("Connection succeed");
        });

    println!("About to create the stream and write to it...");
    tokio::run(client);
    println!("Stream has been created and written to.");
}
