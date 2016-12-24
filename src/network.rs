extern crate mio;
extern crate chrono;
extern crate postgres;
extern crate byteorder;
extern crate rand;
// extern crate time;

use self::byteorder::{ByteOrder, BigEndian};

use util::{NBYTES_U64, NBYTES_U32};

use self::mio::*;
use self::mio::tcp::{TcpListener, TcpStream};

use self::chrono::*;

use self::postgres::{Connection, TlsMode};

use std::collections::{HashMap};
use std::time;
use std::io::{Read, Write};
use std::net;

use std::thread;

use self::rand::{thread_rng, Rng};

const LOCALHOST: &'static str = "127.0.0.1";
const SERVER_TOKEN: Token = Token(0);
const SERVER_DEFAULT_PORT: &'static str = "9001";

#[derive(Clone, Debug)]
pub struct Msg
{
    pub magic:      u32,
    pub command:    [u8; 12],
    pub length:     u32,
    pub checksum:   [u8; 4],
    pub payload:    Vec<u8>
}

#[derive(Debug)]
pub struct Peer
{
    pub id:         i32,
    pub addr:       String,
    pub port:       i16,
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

struct NetworkManager
{
    peer_history: Vec<Peer>,
    peers: Vec<Peer>,
}

impl NetworkManager
{
    pub fn new() -> NetworkManager
    {
        NetworkManager
        {
            peer_history: vec![],
            peers: vec![],
        }
    }
}

pub fn bootstrap(
    server_addr: &str,
    server_port: &str,
    peer_history: Vec<Peer>) -> Vec<Peer>
{
    let mut peers = vec![];
    for peer in peer_history
    {
        if peer.port.to_string() != server_port
        {
            let addr = peer.addr.clone() + ":" + &peer.port.to_string();
            println!("Connecting to peer at {}", addr);

            match net::TcpStream::connect((peer.addr.as_str(), peer.port as u16))
            {
                Ok(mut stream) => {
                    println!("Connected to {:?}", stream.peer_addr().unwrap());

                    snd_addp(server_addr, server_port, &mut stream);
                    snd_lisp(server_addr, server_port, &mut stream);

                    peers.push(
                        Peer {
                            id: peer.id,
                            addr: peer.addr,
                            port: peer.port,
                            timestamp: UTC::now().timestamp(),
                            socket: Some(stream)
                        });
                }
                Err(e) => {
                    println!("Error connecting to host: {}", e);
                }
            }
        }
    }
    peers
}

pub fn start_server(port: Option<String>)
{
    let port = match port
    {
        Some(port) => {
            port
        }
        None => {
            SERVER_DEFAULT_PORT.to_string()
        }
    };
    let addr = (LOCALHOST.to_string() + ":" + &port).parse().expect("Failed to parse server addr");
    let poll = Poll::new().expect("Failed to create poll");
    let server = TcpListener::bind(&addr).unwrap();

    poll.register(
        &server,
        SERVER_TOKEN,
        Ready::readable(),
        PollOpt::level()).expect("Failed to register server socket");

    let mut token_counter: usize = 0;
    let mut clients: HashMap<Token, TcpStream> = HashMap::new();
    let mut events = Events::with_capacity(1024);

    println!("Listening on {}", addr);

    let db_url = "postgresql://chain@localhost:5555/chaindb";
    let db = Connection::connect(db_url, TlsMode::None).expect("Unable to connect to database");

    let peer_history = db.query(
        "SELECT id, addr, port, timestamp FROM peers ORDER BY timestamp DESC;",
        &[])
        .unwrap()
        .iter()
        .map(|row| Peer {
            id: row.get(0),
            addr: row.get(1),
            port: row.get(2),
            timestamp: row.get(3),
            socket: None
        })
        .collect();
    println!("{:#?}", peer_history);
    let mut peers = bootstrap(LOCALHOST, &port, peer_history);

    for peer in &peers
    {
        db.execute(
            "UPDATE peers SET timestamp = $1 WHERE id = $2",
            &[&peer.timestamp, &peer.id])
            .unwrap();
    }

    loop
    {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter()
        {
            println!("EVENT");
            match event.token()
            {
                SERVER_TOKEN => {
                    println!("handle connection");
                    handle_connection(
                        &server,
                        &mut token_counter,
                        &poll,
                        &mut clients);
                }

                token => {
                    println!("handle message");
                    handle_message(
                        &server,
                        token,
                        &mut clients,
                        &mut peers,
                        &db
                    );
                }
            }
        }
    }
}

fn handle_connection(
    server: &TcpListener,
    token_counter: &mut usize,
    poll: &Poll,
    clients: &mut HashMap<Token, TcpStream>)
{
    let socket = match server.accept()
    {
        Err(e) => {
            println!("Error accepting connection: {}", e);
            return;
        }
        Ok((socket, _)) => { socket }
    };

    println!("Accepted connection from {:?}", socket.peer_addr().unwrap());

    *token_counter += 1;
    let token = Token(*token_counter);

    poll.register(
        &socket,
        token,
        Ready::readable(),
        PollOpt::edge()).expect("Failed to register client socket");

    clients.insert(token, socket);
}

fn handle_message(
    server: &TcpListener,
    token: Token,
    clients: &mut HashMap<Token, TcpStream>,
    peers: &mut Vec<Peer>,
    db: &Connection)
{
    // let mut rng = rand::thread_rng();
    // let stutter = time::Duration::from_millis(rng.gen_range::<u64>(0, 5000));
    // thread::sleep(stutter);

    let mut mgc = [0; 4];
    let mut cmd = [0; 4];
    let mut len = [0; 4];
    let mut sum = [0; 4];

    let mut stream = &clients[&token];

    loop
    {
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
                                        let paylen = BigEndian::read_u32(&len) as u64;
                                        let mut pay = vec![];
                                        let mut take = stream.take(paylen);
                                        let _ = take.read_to_end(&mut pay);

                                        let msg = String::from_utf8(pay.clone()).unwrap();

                                        println!("-- {} {}", String::from_utf8(cmd.clone().to_vec()).unwrap(), &msg);
                                        match &cmd
                                        {
                                            b"addp" => {
                                                rcv_addp(
                                                    msg,
                                                    server,
                                                    peers,
                                                    db);
                                            }
                                            b"remp" => {
                                                rcv_remp(
                                                    msg,
                                                    peers);
                                            }
                                            b"lisp" => {
                                                rcv_lisp(
                                                    msg,
                                                    peers);
                                            }
                                            b"resp" => {
                                                match &pay[..4]
                                                {
                                                    b"lisp" =>
                                                    {
                                                        let mut msg = String::from_utf8(pay[4..].to_vec()).unwrap();
                                                        rcv_resp_lisp(
                                                            msg,
                                                            server,
                                                            peers,
                                                            db);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            b"blnc" => {
                                                print!(" balance\n");
                                            }
                                            b"addt" => {
                                                print!(" add transaction\n");
                                            }
                                            b"vdlt" => {
                                                print!(" validate transaction\n");
                                            }
                                            b"pent" => {
                                                print!(" pending transactions\n");
                                            }
                                            b"lisb" => {
                                                print!(" get blocks\n");
                                            }
                                            b"getb" => {
                                                print!(" get block\n");
                                            }
                                            b"addb" => {
                                                print!(" add block\n");
                                            }
                                            b"geth" => {
                                                print!(" get block height\n");
                                            }
                                            b"getl" => {
                                                print!(" get latest block\n");
                                            }
                                            b"chat" => {
                                                print!(" chat\n");
                                            }
                                            b"echo" => {
                                                print!(" echo\n");
                                            }
                                            _ => {
                                                print!(" unknown cmd\n");
                                            }
                                        }
                                    }
                                    Err(e) =>
                                    {
                                        println!("Error reading client stream (sum): {}", e);
                                        break;
                                    }
                                }
                            }
                            Err(e) =>
                            {
                                println!("Error reading client stream (len): {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) =>
                    {
                        println!("Error reading client stream (cmd): {}", e);
                        break;
                    }
                }
            }
            Err(e) =>
            {
                println!("Error reading client stream (mgc): {}", e);
                break;
            }
        }
    }
}

fn snd_addp(
    addr: &str,
    port: &str,
    stream: &mut net::TcpStream)
{
    let mut mgc = [0; NBYTES_U32];
    BigEndian::write_u32(&mut mgc, 0);
    let mut cmd = b"addp";
    let mut len = [0; NBYTES_U32];
    let mut sum = b"    ";
    let mut pay = addr.to_string() + ":" + port;
    BigEndian::write_u32(&mut len, pay.len() as u32);
    let mut msg = vec![];
    msg.extend_from_slice(&mgc);
    msg.extend_from_slice(cmd);
    msg.extend_from_slice(&len);
    msg.extend_from_slice(sum);
    msg.extend_from_slice(pay.as_bytes());

    match stream.write(&msg)
    {
        Ok(nbytes) => {
            println!("Bytes written: {}", nbytes);
        }
        Err(e) => {
            println!("Error writing to stream: {}", e);
        }
    }
    stream.flush();
}

fn rcv_addp(
    payload: String,
    server: &TcpListener,
    peers: &mut Vec<Peer>,
    db: &Connection) -> bool
{
    println!("rcv_addp {}", payload);
    match payload.find(":")
    {
        Some(port_idx) =>
        {
            let ip = payload.split_at(port_idx).0.to_owned();
            let port = payload.split_at(port_idx+1).1.parse::<i16>().unwrap();
            let timestamp = UTC::now().timestamp();

            if  ip != server.local_addr().unwrap().ip().to_string() ||
                port != server.local_addr().unwrap().port() as i16
            {
                match net::TcpStream::connect((ip.as_str(), port as u16))
                {
                    Ok(mut stream) =>
                    {
                        let trans = db.transaction().unwrap();
                        trans.execute("LOCK TABLE peers IN SHARE ROW EXCLUSIVE MODE;", &[]).unwrap();
                        trans.execute(
                            "WITH upsert AS
                            (UPDATE peers SET timestamp = $3 WHERE addr = $1 AND port = $2 RETURNING *)
                            INSERT INTO peers (addr, port, timestamp) SELECT $1, $2, $3
                            WHERE NOT EXISTS (SELECT * FROM upsert);",
                            &[&ip, &port, &timestamp])
                            .unwrap();
                        trans.commit().unwrap();

                        match peers.iter().find(|p| p.addr == ip && p.port == port)
                        {
                            Some(_) => {}
                            None =>
                            {
                                let peer = Peer {
                                    id: 0,
                                    addr: ip,
                                    port: port,
                                    timestamp: timestamp,
                                    socket: Some(stream)
                                };

                                peers.push(peer);
                            }
                        }
                    }
                    Err(e) => {}
                }
            }
        }
        None => { return false; }
    }
    return true;
}

fn rcv_remp(
    payload: String,
    peers: &mut Vec<Peer>)
{
    println!("rcv_remp {}", payload);
    match payload.find(':')
    {
        Some(port_idx) => {
            let ip = payload.split_at(port_idx).0.to_owned();
            let port = payload.split_at(port_idx+1).1.parse::<i16>().unwrap();

            match peers.iter_mut().position(|p| p.addr == ip && p.port == port)
            {
                Some(peer_idx) =>
                {
                    peers.remove(peer_idx);
                }
                None => {}
            }
        }
        None => {
            println!("Couldn't parse port");
        }
    }
}

fn snd_lisp(
    addr: &str,
    port: &str,
    stream: &mut net::TcpStream)
{
    let mut mgc = [0; NBYTES_U32];
    BigEndian::write_u32(&mut mgc, 0);
    let mut cmd = b"lisp";
    let mut len = [0; NBYTES_U32];
    let mut sum = b"    ";
    let mut pay = addr.to_string() + ":" + port;
    BigEndian::write_u32(&mut len, pay.len() as u32);
    let mut msg = vec![];
    msg.extend_from_slice(&mgc);
    msg.extend_from_slice(cmd);
    msg.extend_from_slice(&len);
    msg.extend_from_slice(sum);
    msg.extend_from_slice(pay.as_bytes());

    match stream.write(&msg)
    {
        Ok(nbytes) => {
            println!("bytes written: {}", nbytes);
        }
        Err(e) => {
            println!("Error writing to stream: {}", e);
        }
    }
    stream.flush();
}

fn rcv_lisp(
    payload: String,
    peers: &mut Vec<Peer>)
{
    println!("rcv_lisp {}", payload);
    match payload.find(':')
    {
        Some(port_idx) => {
            let ip = payload.split_at(port_idx).0.to_owned();
            let port = payload.split_at(port_idx+1).1.parse::<i16>().unwrap();

            match peers.iter_mut().position(|p| p.addr == ip && p.port == port)
            {
                Some(peer_idx) =>
                {
                    let mut stream = peers[peer_idx].clone().socket.unwrap();

                    let mut mgc = [0; NBYTES_U32];
                    BigEndian::write_u32(&mut mgc, 0);
                    let mut cmd = b"resp";
                    let mut len = [0; NBYTES_U32];
                    let mut sum = b"    ";
                    let mut pay = "lisp".to_owned();
                    for peer in peers.clone()
                    {
                        pay.push_str(&peer.addr);
                        pay.push_str(":");
                        pay.push_str(&peer.port.to_string());
                        pay.push_str(",");
                    }
                    BigEndian::write_u32(&mut len, pay.len() as u32);

                    let mut msg = vec![];
                    msg.extend_from_slice(&mgc);
                    msg.extend_from_slice(cmd);
                    msg.extend_from_slice(&len);
                    msg.extend_from_slice(sum);
                    msg.extend_from_slice(pay.as_bytes());

                    match stream.write(&msg)
                    {
                        Ok(nbytes) => {
                            println!("bytes written: {} at {}", nbytes, UTC::now().time());
                        }
                        Err(e) => {
                            println!("Error writing to stream: {}", e);
                        }
                    }
                    match stream.take_error()
                    {
                        Ok(Some(err)) =>
                        {
                            println!("{:?}", err);
                        }
                        _ => {}
                    }
                    stream.flush();
                }
                None => {}
            }
        }
        None => {}
    }
}

fn rcv_resp_lisp(
    payload: String,
    server: &TcpListener,
    peers: &mut Vec<Peer>,
    db: &Connection)
{
    println!("rcv_resp_lisp {}", payload);
    let mut msg = payload.clone();
    loop
    {
        match msg.find(',')
        {
            Some(idx) => {
                let temp = msg.clone();
                let addr = temp.split_at(idx).0;
                msg = msg[idx+1..].to_owned();

                if !rcv_addp(
                    addr.to_owned(),
                    server,
                    peers,
                    db)
                {
                    break;
                }
            }
            None => { break; }
        }
    }
}
