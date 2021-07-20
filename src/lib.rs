use std::fmt;
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};
use std::usize;
pub struct Connection {
    pub stream: TcpStream,
    pub connect_time: u64,
}

impl Connection{
    pub fn new(stream: TcpStream) -> Self {
        let s = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        return Connection {
            stream,
            connect_time: s,
        };
    }
}


pub struct StackItem {
    pub size:usize,
    pub data:Vec<u8>,
}


impl StackItem {
    pub fn new(size: usize, data: Vec<u8>) -> Self {
        return StackItem { size, data };
    }
}

impl fmt::Display for StackItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "size: {}; data: <{}>", self.size, String::from_utf8_lossy(&self.data))
    }
}
