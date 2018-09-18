use cardano_storage::types::HASH_SIZE;
use cardano_storage::{tag};
use cardano::util::{hex};
use std::sync::{Arc};
use config::{Networks};

use iron;
use iron::{Request, Response, IronResult};
use iron::status;

use router;
use router::{Router};

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
        router.get(":network/pack/:packid", self, "pack")
    }
}

impl iron::Handler for Handler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref network_name = req.extensions.get::<router::Router>().unwrap().find("network").unwrap();

        if ! common::validate_network_name (network_name) {
            return Ok(Response::with(status::BadRequest));
        }

        let net = match self.networks.get(network_name.to_owned()) {
            None => return Ok(Response::with(status::BadRequest)),
            Some(net) => net
        };
        let ref packid = req.extensions.get::<router::Router>().unwrap().find("packid").unwrap();
        if ! packid.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            error!("invalid packid: {}", packid);
            return Ok(Response::with(status::BadRequest));
        }
        info!("query pack: {}", packid);
        let packhash_vec = match tag::read(&net.storage, &packid) {
            None => hex::decode(&packid).unwrap(),
            Some(t) => t
        };

        let mut packhash = [0;HASH_SIZE];
        packhash[..].clone_from_slice(packhash_vec.as_slice());
        let path = net.storage.config.get_pack_filepath(&packhash);

        Ok(Response::with((status::Ok, path)))
    }
}
