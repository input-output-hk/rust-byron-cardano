use config::Config;
use storage::{Storage, tag, block_location, block_read_location};
use wallet_crypto::{cbor};
use wallet_crypto::util::{hex};
use blockchain;
use std::sync::{Arc};

use iron;
use iron::{Request, Response, IronResult};
use iron::status;

use router;
use router::{Router};

use handlers::common;

pub struct Handler {
    storage: Arc<Storage>
}
impl Handler {
    pub fn new(storage: Arc<Storage>) -> Self {
        Handler {
            storage: storage
        }
    }
    pub fn route(self, router: &mut Router) -> &mut Router {
        router.get(":network/block/:blockid", self, "block")
    }
}

impl iron::Handler for Handler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref network_name = req.extensions.get::<router::Router>().unwrap().find("network").unwrap();

        if ! common::validate_network_name (network_name) {
            return Ok(Response::with(status::BadRequest));
        }

        let ref blockid = req.extensions.get::<router::Router>().unwrap().find("blockid").unwrap();
        if ! blockid.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            error!("invalid blockid: {}", blockid);
            return Ok(Response::with(status::BadRequest));
        }
        let hh_bytes = match tag::read(&self.storage, &blockid) {
            None => hex::decode(&blockid).unwrap(),
            Some(t) => t
        };
        let hh = blockchain::HeaderHash::from_slice(&hh_bytes).expect("blockid invalid");
        info!("querying block header: {}", hh);

        match block_location(&self.storage, hh.bytes()) {
            None => {
                warn!("block `{}' does not exist", hh);
                Ok(Response::with((status::NotFound, "Not Found")))
            },
            Some(loc) => {
                debug!("blk location: {:?}", loc);
                match block_read_location(&self.storage, &loc, hh.bytes()) {
                    None        => {
                        error!("error while reading block at location: {:?}", loc);
                        Ok(Response::with(status::InternalServerError))
                    },
                    Some(bytes) => {
                        let blk : blockchain::Block = cbor::decode_from_cbor(&bytes).unwrap();
                        let hdr = blk.get_header();
                        Ok(Response::with((status::Ok, bytes)))
                    }
                }
            }
        }
    }
}
