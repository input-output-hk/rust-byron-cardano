use storage;
use storage::{Storage, tag};

use blockchain;

use wallet_crypto::util::{hex};
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
        router.get(":network/epoch/:epochid", self, "epochid")
    }
}

impl iron::Handler for Handler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref network_name = req.extensions.get::<router::Router>().unwrap().find("network").unwrap();

        if ! common::validate_network_name (network_name) {
            return Ok(Response::with(status::BadRequest));
        }

        let ref epochid_str = req.extensions.get::<router::Router>().unwrap().find("epochid").unwrap();

        if ! epochid_str.chars().all(|c| c.is_digit(10)) {
            error!("invalid epochid: {}", epochid_str);
            return Ok(Response::with(status::BadRequest));
        }

        let epochid = epochid_str.parse::<blockchain::EpochId>().unwrap();
        let opackref = storage::epoch::epoch_read_pack(&self.storage.config, epochid);
        match opackref {
            Err(_) => {
                return Ok(Response::with(status::NotFound));
            },
            Ok(packref) => {
                let path = self.storage.config.get_pack_filepath(&packref);
                Ok(Response::with((status::Ok, path)))
            },
        }
    }
}
