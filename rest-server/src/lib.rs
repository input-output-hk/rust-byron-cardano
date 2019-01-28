mod error;
mod server_service;

pub use self::error::Error;
pub use self::server_service::ServerService;
pub use actix_web;
pub use futures;
pub use native_tls;

type ServerResult<T> = Result<T, Error>;
