use config::{Networks};
use storage::{block_location, block_read_location};
use std::sync::{Arc};

use iron;
use iron::{Request, Response, IronResult};
use iron::status;

use router;
use router::{Router};

use handlers::common;
use exe_common::utils::*;

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
        router.get(":network/tip", self, "block")
    }
}

impl iron::Handler for Handler {
    // XXX
    //
    // The current implementation of the TIP handler is to look for the latest epoch
    // and to extract its latest block
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref network_name = req.extensions.get::<router::Router>().unwrap().find("network").unwrap();

        if ! common::validate_network_name (network_name) {
            return Ok(Response::with(status::BadRequest));
        }

        let net = match self.networks.get(network_name.to_owned()) {
            None => return Ok(Response::with(status::BadRequest)),
            Some(net) => net
        };
        let net_cfg = &net.config;

        let hh =
            match find_earliest_epoch(&net.storage, net_cfg.epoch_start, 100) {
                None => return Ok(Response::with((status::NotFound, "No Tip To Serve"))),
                Some((_, packhash)) =>
                    get_last_blockid(&net.storage.config, &packhash).unwrap(),
            };

        match block_location(&net.storage, hh.bytes()) {
            None => {
                warn!("block `{}' does not exist", hh);
                Ok(Response::with((status::NotFound, "Not Found")))
            },
            Some(loc) => {
                debug!("blk location: {:?}", loc);
                match block_read_location(&net.storage, &loc, hh.bytes()) {
                    None        => {
                        error!("error while reading block at location: {:?}", loc);
                        Ok(Response::with(status::InternalServerError))
                    },
                    Some(rblk) => {
                        Ok(Response::with((status::Ok, rblk.to_header().as_ref())))
                    }
                }
            }
        }
    }
}

