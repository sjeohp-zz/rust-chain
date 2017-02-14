extern crate mio;
use std::net;

#[derive(Debug)]
pub struct Peer
{
    pub ip:       String,
    pub port:       i32,
    pub timestamp:  i64,
    pub socket:     Option<net::TcpStream>
}

impl Clone for Peer
{
    fn clone(&self) -> Peer
    {
        Peer {
            ip: self.ip.clone(),
            port: self.port,
            timestamp: self.timestamp,
            socket: match self.socket
            {
                Some(ref socket) =>
                {
                    match socket.try_clone()
                    {
                        Ok(socket) => { Some(socket) }
                        Err(_) => { None }
                    }
                }
                None => { None }
            }
        }
    }
}

impl Peer
{
    pub fn new(
        ip: String,
        port: i32,
        timestamp: i64,
        socket: Option<net::TcpStream>) -> Peer
    {
        Peer {
            ip: ip,
            port: port,
            timestamp: timestamp,
            socket: socket
        }
    }
}
