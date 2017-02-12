extern crate mio;
extern crate byteorder;
extern crate rand;

use self::byteorder::{ByteOrder, LittleEndian};

use util::{NBYTES_U32};

use self::mio::tcp::{TcpStream};
use std::io::{Error, Read};

use peer::*;

#[derive(Clone, Debug)]
pub struct Msg
{
    pub magic:      u32,
    pub command:    [u8; 12],
    pub length:     u32,
    pub checksum:   [u8; 4],
    pub payload:    Vec<u8>
}

impl Msg
{
    pub fn from_stream(stream: &mut TcpStream) -> Result<Msg, Error>
    {
        let mut mgc = [0; 4];
        let mut cmd = [0; 12];
        let mut len = [0; 4];
        let mut sum = [0; 4];

        match stream.read_exact(&mut mgc)
        {
            Ok(_) =>
            {
                match stream.read_exact(&mut cmd)
                {
                    Ok(_) =>
                    {
                        match stream.read_exact(&mut len)
                        {
                            Ok(_) =>
                            {
                                match stream.read_exact(&mut sum)
                                {
                                    Ok(_) =>
                                    {
                                        let paylen = LittleEndian::read_u32(&len);
                                        let mut pay = vec![];
                                        let mut take = stream.take(paylen as u64);
                                        let _ = take.read_to_end(&mut pay);
                                        let mut msg = Msg {
                                            magic: LittleEndian::read_u32(&mgc),
                                            command: [0; 12],
                                            length: paylen,
                                            checksum: sum,
                                            payload: pay
                                        };
                                        msg.command.clone_from_slice(&cmd);
                                        Ok(msg)
                                    }
                                    Err(e) =>
                                    {
                                        Err(e)
                                    }
                                }
                            }
                            Err(e) =>
                            {
                                Err(e)
                            }
                        }
                    }
                    Err(e) =>
                    {
                        Err(e)
                    }
                }
            }
            Err(e) =>
            {
                Err(e)
            }
        }
    }

    pub fn to_vec(&self) -> Vec<u8>
    {
        let mut mgc = [0; NBYTES_U32];
        LittleEndian::write_u32(&mut mgc, self.magic);
        let mut len = [0; NBYTES_U32];
        LittleEndian::write_u32(&mut len, self.length);
        let mut msg = vec![];
        msg.extend_from_slice(&mgc);
        msg.extend_from_slice(&self.command);
        msg.extend_from_slice(&len);
        msg.extend_from_slice(&self.checksum);
        msg.extend_from_slice(&self.payload);
        msg
    }

    pub fn new_add_peer(addr: &str, port: &str) -> Msg
    {
        let pay = addr.to_string() + ":" + port;
        let mut msg = Msg {
            magic:      0u32,
            command:    [0; 12],
            length:     pay.len() as u32,
            checksum:   [0; 4],
            payload:    pay.as_bytes().to_vec()
        };
        msg.command.clone_from_slice(b"addp        ");
        msg
    }

    pub fn new_remove_peer(addr: &str, port: &str) -> Msg
    {
        let pay = addr.to_string() + ":" + port;
        let mut msg = Msg {
            magic:      0u32,
            command:    [0; 12],
            length:     pay.len() as u32,
            checksum:   [0; 4],
            payload:    pay.as_bytes().to_vec()
        };
        msg.command.clone_from_slice(b"remp        ");
        msg
    }

    pub fn new_list_peers(addr: &str, port: &str) -> Msg
    {
        let pay = addr.to_string() + ":" + port;
        let mut msg = Msg {
            magic:      0u32,
            command:    [0; 12],
            length:     pay.len() as u32,
            checksum:   [0; 4],
            payload:    pay.as_bytes().to_vec()
        };
        msg.command.clone_from_slice(b"lisp        ");
        msg
    }

    pub fn new_list_peers_response(peers: &Vec<Peer>) -> Msg
    {
        let mut pay = "lisp        ".to_owned();
        for peer in peers.iter()
        {
            pay.push_str(&peer.addr.clone());
            pay.push_str(":");
            pay.push_str(&peer.port.clone().to_string());
            pay.push_str(",");
        }
        let mut msg = Msg {
            magic:      0u32,
            command:    [0; 12],
            length:     pay.len() as u32,
            checksum:   [0; 4],
            payload:    pay.as_bytes().to_vec()
        };
        msg.command.clone_from_slice(b"resp        ");
        msg
    }

    pub fn new_add_transaction(bytes: Vec<u8>) -> Msg
    {
        let pay = bytes;
        let mut msg = Msg {
            magic:      0u32,
            command:    [0; 12],
            length:     pay.len() as u32,
            checksum:   [0; 4],
            payload:    pay
        };
        msg.command.clone_from_slice(b"addt        ");
        msg
    }

    pub fn new_add_block(bytes: Vec<u8>) -> Msg
    {
        let pay = bytes;
        let mut msg = Msg {
            magic:      0u32,
            command:    [0; 12],
            length:     pay.len() as u32,
            checksum:   [0; 4],
            payload:    pay
        };
        msg.command.clone_from_slice(b"addb        ");
        msg
    }

    pub fn new_get_block(bytes: Vec<u8>) -> Msg
    {
        let pay = bytes;
        let mut msg = Msg {
            magic:      0u32,
            command:    [0; 12],
            length:     pay.len() as u32,
            checksum:   [0; 4],
            payload:    pay
        };
        msg.command.clone_from_slice(b"getb        ");
        msg
    }
}
