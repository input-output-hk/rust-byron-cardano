use config::{Networks};
use cardano_storage::{Error, tag};
use std::sync::{Arc};

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
        router.get(":network/tip", self, "tip")
    }
}

impl iron::Handler for Handler {
    // XXX
    //
    // The current implementation of the TIP handler is to look for the HEAD tag
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref network_name = req.extensions.get::<router::Router>().unwrap().find("network").unwrap();

        if ! common::validate_network_name (network_name) {
            return Ok(Response::with(status::BadRequest));
        }

        let net = match self.networks.get(network_name.to_owned()) {
            None => return Ok(Response::with(status::BadRequest)),
            Some(net) => net
        };

        match net.storage.get_block_from_tag(&tag::HEAD) {
            Err(Error::NoSuchTag) =>
                Ok(Response::with((status::NotFound, "No Tip To Serve"))),
            Err(err) => {
                error!("error while reading block: {:?}", err);
                Ok(Response::with(status::InternalServerError))
            },
            Ok(block) => {
                Ok(Response::with((status::Ok, block.get_header().to_raw().as_ref())))
            }
        }
    }
}

