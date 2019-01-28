use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    BindFailed(IoError),
    ServerAlreadyStopped,
    ServerStopTimeout,
    ServerStopFailed,
}
