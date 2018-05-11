use storage;
use storage::{Storage, tag};
use wallet_crypto::util::{hex};
use std::sync::{Arc};

use iron;
use iron::{Request, Response, IronResult};
use iron::status;

use router;
use router::{Router};

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
        router.get("/pack/:packid", self, "pack")
    }
}

impl iron::Handler for Handler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref packid = req.extensions.get::<router::Router>().unwrap().find("packid").unwrap();
        info!("query pack: {}", packid);
        let packhash_vec = match tag::read(&self.storage, &packid) {
            None => hex::decode(&packid).unwrap(),
            Some(t) => t
        };

        let mut packhash = [0;storage::types::HASH_SIZE];
        packhash[..].clone_from_slice(packhash_vec.as_slice());
        let path = self.storage.config.get_pack_filepath(&packhash);

        Ok(Response::with((status::Ok, path)))
    }
}