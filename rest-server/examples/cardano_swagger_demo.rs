use rest_server::actix_web::error::Error as ActixError;
use rest_server::actix_web::{App, FutureResponse, HttpResponse, Json, Query, Responder, State};
use rest_server::futures::{Async, Future, Poll};
use rest_server::native_tls::{Identity, TlsAcceptor};
use rest_server::ServerService;
use serde_derive::Deserialize;
use serde_json::json;
use std::io::{stdin, BufRead};
use std::sync::{Arc, Mutex};

type CounterState = Arc<Mutex<u64>>;

fn main() {
    let state = Arc::new(Mutex::new(0));
    let tls = load_tls_acceptor();
    let handler = move || {
        App::with_state(state.clone())
            .prefix("api")
            .scope("v1", |scope| {
                scope
                    .resource("/next-update", |r| r.get().with(next_update_v1))
                    .resource("/restart-node", |r| r.get().with(restart_node_v1))
                    .resource("/counter", |r| r.get().with(counter_v1))
            })
    };

    let server_handler = ServerService::start("127.0.0.1:8088", tls, handler).unwrap();

    stdin()
        .lock()
        .lines()
        .filter_map(|res| res.ok())
        .filter(|line| line == "stop")
        .next();
    println!("Stopping...");
    match server_handler.stop().wait() {
        Ok(_) => println!("Ok"),
        Err(e) => println!("Failed: {:?}", e),
    };
}

fn load_tls_acceptor() -> TlsAcceptor {
    let identity_pkcs12 = include_bytes!("example_identity.p12");
    let identity = Identity::from_pkcs12(identity_pkcs12, "").unwrap();
    TlsAcceptor::new(identity).unwrap()
}

fn next_update_v1(_: ()) -> impl Responder {
    Json(json!({
      "data": {
        "applicationName": "string",
        "version": 0
      },
      "meta": {
        "pagination": {}
      },
      "status": "success"
    }))
}

#[derive(Deserialize)]
struct RestartNodeV1Params {
    #[serde(default)]
    pub force_ntp_check: bool,
}

fn restart_node_v1(params: Query<RestartNodeV1Params>) -> impl Responder {
    println!("Restart! force_ntp_check = {}", params.force_ntp_check);
    HttpResponse::Ok()
}

struct CounterFuture {
    state: CounterState,
}

impl Future for CounterFuture {
    type Item = String;
    type Error = ActixError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut counter = self.state.lock().unwrap_or_else(|e| e.into_inner());
        *counter += 1;
        let message = format!("Call no. {}", counter);
        println!("{}", message);
        Ok(Async::Ready(message))
    }
}

fn counter_v1(state: State<CounterState>) -> impl Responder {
    let future = CounterFuture {
        state: state.clone(),
    };
    Box::new(future) as FutureResponse<_>
}
