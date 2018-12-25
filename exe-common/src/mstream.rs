use std::fmt;
use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, SystemTime};

use network::{Error, Result};

pub struct MetricStart {
    bytes_start: u64,
    started: SystemTime,
}

impl MetricStart {
    pub fn new(sz: u64) -> Self {
        let time_start = SystemTime::now();
        MetricStart {
            bytes_start: sz,
            started: time_start,
        }
    }

    pub fn diff(&self, end_sz: u64) -> MetricStats {
        let duration = self.started.elapsed().unwrap();
        MetricStats {
            bytes_transfered: end_sz - self.bytes_start,
            duration: duration,
        }
    }
}

pub struct MetricStats {
    bytes_transfered: u64,
    duration: Duration,
}

fn size_print(bytes: u64) -> String {
    if bytes > 1024 * 1024 {
        format!("{:.1} mb", bytes as f64 / (1024 * 1024) as f64)
    } else if bytes > 2048 {
        format!("{:.2} kb", bytes as f64 / 1024 as f64)
    } else {
        format!("{}  b", bytes)
    }
}

impl fmt::Display for MetricStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let x = self.duration.as_secs() * 1_000_000_000 + self.duration.subsec_nanos() as u64;
        let s = self.bytes_transfered * 1_000_000_000 / x;
        write!(
            f,
            "{} bytes transfered in {}.{:03} seconds. {}/s",
            self.bytes_transfered,
            self.duration.as_secs(),
            self.duration.subsec_millis(),
            size_print(s)
        )
    }
}

pub struct MStream {
    //lock: RwLock,
    stream: TcpStream,
    read_sz: u64,
    write_sz: u64,
}

const TIMEOUT_SECONDS: u64 = 30;
const TIMEOUT_NANO_SECONDS: u32 = 0;

impl MStream {
    pub fn init(dest: &SocketAddr) -> Result<Self> {
        let timeout = Duration::new(TIMEOUT_SECONDS, TIMEOUT_NANO_SECONDS);
        let stream = match TcpStream::connect_timeout(dest, timeout) {
            Ok(stream) => stream,
            Err(ioerr) => {
                return if ioerr.kind() == io::ErrorKind::TimedOut {
                    Err(Error::ConnectionTimedOut)
                } else {
                    Err(Error::from(ioerr))
                };
            }
        };
        stream.set_nodelay(true)?;
        //let lock = RwLock::new(5);
        Ok(MStream {
            //lock: lock,
            stream: stream,
            read_sz: 0,
            write_sz: 0,
        })
    }

    pub fn get_read_sz(&self) -> u64 {
        self.read_sz
    }

    pub fn get_write_sz(&self) -> u64 {
        self.write_sz
    }
}

impl Read for MStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let sz = self.stream.read(buf)?;
        self.read_sz += sz as u64;
        Ok(sz)
    }
}

impl Write for MStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let sz = self.stream.write(buf)?;
        self.write_sz += sz as u64;
        Ok(sz)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}
