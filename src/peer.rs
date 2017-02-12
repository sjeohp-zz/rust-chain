extern crate mio;
use std::net;

#[derive(Debug)]
pub struct Peer
{
    pub id:         i32,
    pub addr:       String,
    pub port:       i32,
    pub timestamp:  i64,
    pub socket:     Option<net::TcpStream>
}

impl Clone for Peer
{
    fn clone(&self) -> Peer
    {
        Peer {
            id: self.id,
            addr: self.addr.clone(),
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
