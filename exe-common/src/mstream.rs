use std::net::TcpStream;
use std::time::{SystemTime, Duration};
use std::sync::{Arc, RwLock};
use std::io::{Read,Write};
use std::io;
use std::fmt;

pub struct MetricStart {
    bytes_start: u64,
    started: SystemTime,
}

impl MetricStart {
    pub fn new(sz: u64) -> Self {
        let time_start = SystemTime::now();
        MetricStart { bytes_start: sz, started: time_start }
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

impl fmt::Display for MetricStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} bytes transfered in {}.{:03} seconds", self.bytes_transfered, self.duration.as_secs(), self.duration.subsec_millis())
    }
}

pub struct MStream {
    //lock: RwLock,
    stream: TcpStream,
    read_sz: u64,
    write_sz: u64,
}

impl MStream {
    pub fn init(dest: &String) -> Self {
        let stream = TcpStream::connect(dest).unwrap();
        stream.set_nodelay(true).unwrap();
        //let lock = RwLock::new(5);
        MStream {
            //lock: lock,
            stream: stream,
            read_sz: 0,
            write_sz: 0,
        }
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
    fn flush(&mut self) -> io::Result<()> { self.stream.flush() }
}