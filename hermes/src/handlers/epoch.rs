use storage;

use std::sync::{Arc};

use iron;
use iron::{Request, Response, IronResult};
use iron::status;

use router;
use router::{Router};

use config::{Networks};
use handlers::common;

pub struct Handler {
    networks: Arc<Networks>
}
impl Handler {
    pub fn new(networks: Arc<Networks>) -> Self {
        Handler {
            networks: networks
        }
    }
    pub fn route(self, router: &mut Router) -> &mut Router {
        router.get(":network/epoch/:epochid", self, "epochid")
    }
}

impl iron::Handler for Handler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref network_name = req.extensions.get::<router::Router>().unwrap().find("network").unwrap();
        let ref epochid_str = req.extensions.get::<router::Router>().unwrap().find("epochid").unwrap();

        if ! common::validate_network_name (network_name) {
            return Ok(Response::with(status::BadRequest));
        }
        let net = match self.networks.get(network_name.to_owned()) {
            None => return Ok(Response::with(status::BadRequest)),
            Some(net) => net
        };

        let epochid = match common::validate_epochid (epochid_str) {
                        None => {
                            error!("invalid epochid: {}", epochid_str);
                            return Ok(Response::with(status::BadRequest));
                        },
                        Some(e) => e,
        };

        let opackref = storage::epoch::epoch_read_pack(&net.storage.config, epochid);
        match opackref {
            Err(_) => {
                return Ok(Response::with(status::NotFound));
            },
            Ok(packref) => {
                let path = net.storage.config.get_pack_filepath(&packref);
                Ok(Response::with((status::Ok, path)))
            },
        }
    }
}
