use config::{Config, Networks};
use exe_common::sync;
use handlers;
use iron;
use router::Router;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

static NETWORK_REFRESH_FREQUENCY: Duration = Duration::from_secs(60 * 10);

pub fn start(cfg: Config) {
    let _refresher = start_networks_refresher(cfg.clone());
    let _server = start_http_server(&cfg, Arc::new(cfg.get_networks().unwrap()));

    // XXX: consider installing a signal handler to initiate a graceful shutdown here
    // XXX: after initiating shutdown, do `refresher.join()` and something similar for `server`.
}

fn start_http_server(cfg: &Config, networks: Arc<Networks>) -> iron::Listening {
    let mut router = Router::new();
    handlers::block::Handler::new(networks.clone()).route(&mut router);
    handlers::pack::Handler::new(networks.clone()).route(&mut router);
    handlers::epoch::Handler::new(networks.clone()).route(&mut router);
    handlers::tip::Handler::new(networks.clone()).route(&mut router);
    info!("listening to port {}", cfg.port);
    iron::Iron::new(router)
        .http(format!("0.0.0.0:{}", cfg.port))
        .expect("start http server")
}

// TODO: make this a struct which receives a shutdown message on a channel and then wraps itself up
fn start_networks_refresher(cfg: Config) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        info!("Refreshing every {:?}", NETWORK_REFRESH_FREQUENCY);
        loop {
            match cfg.get_networks() {
                Err(err) => warn!("Refresh failed: {:?}", err),
                Ok(networks) => {
                    refresh_networks(networks);
                    info!("Refresh completed")
                }
            }
            thread::sleep(NETWORK_REFRESH_FREQUENCY);
        }
    })
}

// XXX: how do we want to report partial failures?
fn refresh_networks(networks: Networks) {
    for (label, net) in networks.into_iter() {
        info!("Refreshing network {:?}", label);
        match Arc::try_unwrap(net.storage) {
            // Cannot just use `.unwrap()` because that requires a debug instance
            Err(_) => warn!(
                "Refresh for network {} failed: Unable to access storage",
                label
            ),
            Ok(storage) => sync::net_sync_native(label, storage),
        }
    }
}
