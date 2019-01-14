extern crate env_logger;
extern crate futures;
extern crate log;
extern crate protocol_tokio;
extern crate tokio;

use protocol_tokio::{Connection, Inbound, InboundStream, Message, OutboundSink};

use futures::{future, sync::mpsc};
use tokio::net::TcpListener;
use tokio::prelude::*;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Parse the address of whatever address we're listening to
    let addr = "127.0.0.1:3000".parse().unwrap();

    let server = TcpListener::bind(&addr)
        .unwrap()
        .incoming()
        .map_err(|err| {
            println!("incoming error = {:?}", err);
        })
        .for_each(move |stream| {
            Connection::accept(stream)
                .map_err(|err| println!("accepting connection error {:?}", err))
                .and_then(|connection| {
                    let (sink, stream): (
                        OutboundSink<_, u8, u8, u8, u8>,
                        InboundStream<_, u8, u8, u8, u8>,
                    ) = connection.split();

                    let (sink_tx, sink_rx) = mpsc::unbounded();

                    let stream = stream
                        .for_each(move |inbound| {
                            match inbound {
                                Inbound::NewNode(lwcid, node_id) => {
                                    sink_tx
                                        .unbounded_send(Message::AckNodeId(lwcid, node_id))
                                        .unwrap();
                                }
                                inbound => {
                                    println!("inbound: {:?}", inbound);
                                }
                            }
                            future::ok(())
                        })
                        .map_err(|err| println!("connection stream error {:#?}", err));

                    let sink = sink_rx
                        .fold(sink, |sink, outbound| match outbound {
                            Message::AckNodeId(_lwcid, node_id) => future::Either::A(
                                sink.ack_node_id(node_id)
                                    .map_err(|err| println!("err {:?}", err)),
                            ),
                            message => future::Either::B(
                                sink.send(message).map_err(|err| println!("err {:?}", err)),
                            ),
                        })
                        .map(|_| ());

                    let connection_task = stream.select(sink).then(|_| {
                        println!("closing connection");
                        Ok(())
                    });

                    tokio::spawn(connection_task)
                })
        })
        .map(|_| {
            println!("stopping to accept new connections");
        });

    println!("About to create the server and wait for connection...");
    tokio::run(server);
}
