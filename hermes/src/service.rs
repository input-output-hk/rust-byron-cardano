use std::sync::Arc;
use iron::Iron;
use handlers;
use router::Router;
use config::Config;

pub fn start(cfg: Config) {
    let mut router = Router::new();
    let networks = Arc::new(cfg.get_networks().unwrap());
    handlers::block::Handler::new(networks.clone()).route(&mut router);
    handlers::pack::Handler::new(networks.clone()).route(&mut router);
    handlers::epoch::Handler::new(networks.clone()).route(&mut router);
    info!("listenting to port {}", cfg.port);
    Iron::new(router)
        .http(format!("0.0.0.0:{}", cfg.port))
        .unwrap();
}
