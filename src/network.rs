extern crate mio;
extern crate chrono;
extern crate postgres;

use self::mio::*;
use self::mio::tcp::{TcpListener, TcpStream};

use self::chrono::*;

use self::postgres::{Connection, TlsMode};

use std::collections::{HashMap};

use std::io::{Read, Write};
use std::net;

use std::thread;

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

#[derive(Clone, Debug)]
pub struct Peer
{
    pub id:         i32,
    pub address:    String,
    pub port:       i16,
    pub timestamp:  i64
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
    server_address: &str,
    server_port: &str,
    peer_history: Vec<Peer>) -> Vec<Peer>
{
    let mut peers = vec![];
    for peer in peer_history
    {
        if peer.port.to_string() != server_port
        {
            let addr = peer.address.clone() + ":" + &peer.port.to_string();
            println!("Connecting to peer at {}", addr);

            match net::TcpStream::connect((peer.address.as_str(), peer.port as u16))
            {
                Ok(mut stream) => {
                    println!("Connected to {:?}", stream.peer_addr().unwrap());
                    let msg = "addp/".to_string() + server_address + ":" + server_port;
                    match stream.write(msg.as_bytes())
                    {
                        Ok(nbytes) => {
                            println!("bytes written: {}", nbytes);
                        }
                        Err(e) => {
                            println!("Error writing to stream: {}", e);
                        }
                    }

                    peers.push(
                        Peer {
                            id: peer.id,
                            address: peer.address,
                            port: peer.port,
                            timestamp: UTC::now().timestamp(),
                        }
                    );
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
    let address = (LOCALHOST.to_string() + ":" + &port).parse().expect("Failed to parse server address");
    let poll = Poll::new().expect("Failed to create poll");
    let server = TcpListener::bind(&address).unwrap();

    poll.register(
        &server,
        SERVER_TOKEN,
        Ready::readable(),
        PollOpt::edge()).expect("Failed to register server socket");

    let mut token_counter: usize = 0;
    let mut clients: HashMap<Token, TcpStream> = HashMap::new();
    let mut events = Events::with_capacity(1024);

    println!("Listening on {}", address);

    let db_url = "postgresql://chain@localhost:5555/chaindb";
    let db = Connection::connect(db_url, TlsMode::None).expect("Unable to connect to database");
    // let exists = db.query("SELECT EXISTS (
    //     SELECT 1
    //     FROM   information_schema.tables
    //     WHERE  table_schema = 'public'
    //     AND    table_name = 'peer_history'
    // )", &[]).unwrap();
    // println!("{:?}", exists);

    let peer_history = db.query(
        "SELECT id, addr, port, timestamp FROM peers ORDER BY timestamp DESC;",
        &[])
        .unwrap()
        .iter()
        .map(|row| Peer {
            id: row.get(0),
            address: row.get(1),
            port: row.get(2),
            timestamp: row.get(3)
        })
        .collect();
    println!("{:?}", peer_history);
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
            match event.token()
            {
                SERVER_TOKEN => {
                    // println!("handle connection");
                    handle_connection(
                        &server,
                        &mut token_counter,
                        &poll,
                        &mut clients);
                }

                token => {
                    // println!("handle message");
                    handle_message(
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

    println!("Accepted connection from {:?}", socket);

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
    token: Token,
    clients: &mut HashMap<Token, TcpStream>,
    peers: &mut Vec<Peer>,
    db: &Connection)
{
    let mut msg = vec![];
    // let mut cmd = [0; 4];

    let mut stream = &clients[&token];
    let _ = stream.read_to_end(&mut msg);
    let msgstr = String::from_utf8(msg.clone()).unwrap();
    println!("{:?}", String::from_utf8(msg.clone()));

    let cmd = msg.split_at(4).0;
    // {
    //     Ok(cmd) =>
    //     {
            println!("{:?}", cmd);
            match cmd
            {
                b"addp" => {
                    print!("add peer\n");

                    match msgstr.find('/')
                    {
                        Some(addr_idx) => {
                            let addr = msgstr.split_at(addr_idx+1).1;
                            match addr.find(':')
                            {
                                Some(port_idx) => {
                                    let ip = addr.split_at(port_idx).0;
                                    let port = addr.split_at(port_idx+1).1;

                                    let peer = Peer {
                                        id: 0,
                                        address: ip.to_owned(),
                                        port: port.parse::<i16>().unwrap(),
                                        timestamp: UTC::now().timestamp()
                                    };

                                    db.execute(
                                        "INSERT INTO peers (id, addr, port, timestamp) VALUES (DEFAULT, $1, $2, $3);",
                                        &[&peer.address, &peer.port, &peer.timestamp])
                                        .unwrap();

                                    peers.push(peer);
                                }
                                None => {
                                    println!("Couldn't parse port");
                                }
                            }
                        }
                        None => {
                            println!("Couldn't parse address");
                        }
                    }
                }
                b"remp" => {
                    print!(" remove peer\n");
                }
                b"lisp" => {
                    print!(" list peers\n");
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
        // }
        // Err(e) =>
        // {
        //     println!("{:?}", cmd);
        //     println!("Error reading client stream: {}", e);
        // }
    // }

}
