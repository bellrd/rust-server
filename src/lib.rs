use std::fmt;
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};
use std::usize;

use uuid::Uuid;
pub struct Connection {
    pub id:Uuid,
    pub stream: TcpStream,
    pub connect_time: u64,
    pub data:Option<Vec<u8>>,
}

impl Connection{
    pub fn new(stream: TcpStream, id:Uuid) -> Self {
        let s = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        return Connection {
            stream,
            connect_time: s,
            data:None,
            id,
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
