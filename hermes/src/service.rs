use config::{Config, Networks};
use exe_common::sync;
use handlers;
use iron;
use router::Router;
use std::sync::Arc;

pub fn start(cfg: Config) {
    let networks = Arc::new(cfg.get_networks().unwrap());
    // start background thread to refresh sync blocks
    start_http_server(cfg, networks);
}

fn start_http_server(cfg: &Config, networks: Arc<Networks>) -> iron::Listening {
    let mut router = Router::new();
    handlers::block::Handler::new(networks.clone()).route(&mut router);
    handlers::pack::Handler::new(networks.clone()).route(&mut router);
    handlers::epoch::Handler::new(networks.clone()).route(&mut router);
    info!("listenting to port {}", cfg.port);
    iron::Iron::new(router)
        .http(format!("0.0.0.0:{}", cfg.port))
        .expect("start http server")
}
}
