extern crate mio;
extern crate chrono;
extern crate postgres;
extern crate byteorder;
extern crate rand;

use self::byteorder::{ByteOrder, BigEndian};

use util::{NBYTES_U64, NBYTES_U32};

use self::mio::*;
use self::mio::channel::{channel, Receiver};
use self::mio::tcp::{TcpListener, TcpStream};

use self::chrono::*;

use self::postgres::{Connection, TlsMode};

use std::collections::{HashMap};
use std::time;
use std::io::{Read, Write};
use std::net;

use std::thread;

use self::rand::{thread_rng, Rng};

use transaction::*;
use peer::*;
use message::*;

const QUIT_TOKEN: Token = Token(0);
const SERVER_TOKEN: Token = Token(1);
const CLIENT_TOKEN_COUNTER: usize = 2;
const LOCALHOST: &'static str = "127.0.0.1";
const SERVER_DEFAULT_PORT: &'static str = "9001";

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

                    let mut addp = Msg::new_add_peer(server_addr, server_port).to_vec();
                    let mut lisp = Msg::new_list_peers(server_addr, server_port).to_vec();

                    let mut tx0 = Tx::new(
                        vec![
                            Txi {
                                src_hash:   [1; 32],
                                src_idx:    2,
                                signature:  [3; 32]
                            }
                        ],
                        vec![
                            Txo {
                                amount: 4,
                                address: [5; 32]
                            }
                        ],
                        6
                    );
                    let mut addt = Msg::new_add_transaction(tx0.to_vec()).to_vec();

                    addp.append(&mut lisp);
                    addp.append(&mut addt);

                    match stream.write(&addp)
                    {
                        Ok(nbytes) => {
                            println!("snd_addp bytes written: {}", nbytes);
                        }
                        Err(e) => {
                            println!("Error writing to stream: {}", e);
                        }
                    }
                    stream.flush();

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

pub fn start_server(port: Option<String>, quit_rcv: Receiver<()>)
{
    let poll = Poll::new().expect("Failed to create poll");

    poll.register(
        &quit_rcv,
        QUIT_TOKEN,
        Ready::readable(),
        PollOpt::level()).expect("Failed to register UI receiver channel");

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
    let server = TcpListener::bind(&addr).unwrap();

    poll.register(
        &server,
        SERVER_TOKEN,
        Ready::readable(),
        PollOpt::level()).expect("Failed to register server socket");

    let mut token_counter: usize = CLIENT_TOKEN_COUNTER;
    let mut clients: HashMap<Token, TcpStream> = HashMap::new();
    let mut events = Events::with_capacity(1024);

    println!("Listening on {}", addr);

    let db_url = "postgresql://chain@localhost:5432/chaindb";
    let db = Connection::connect(db_url, TlsMode::None).expect("Unable to connect to database");

    let peer_history = db.query(
        "SELECT id, ip, port, timestamp FROM peers ORDER BY timestamp DESC;",
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

    let mut peers = bootstrap(LOCALHOST, &port, peer_history);

    for peer in &peers
    {
        db.execute(
            "UPDATE peers SET timestamp = $1 WHERE id = $2",
            &[&peer.timestamp, &peer.id])
            .unwrap();
    }

    'event_loop: loop
    {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter()
        {
            println!("EVENT");
            match event.token()
            {
                QUIT_TOKEN => {
                    println!("handle quit");
                    handle_quit(
                        LOCALHOST,
                        &port,
                        peers);
                    break 'event_loop;
                }

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

fn handle_quit(
    server_ip: &str,
    server_port: &str,
    peers: Vec<Peer>)
{
    for peer in peers
    {
        let remp = Msg::new_remove_peer(server_ip, server_port).to_vec();

        match peer.socket.unwrap().write(&remp)
        {
            Ok(nbytes) => {
                println!("snd_addp bytes written: {}", nbytes);
            }
            Err(e) => {
                println!("Error writing to stream: {}", e);
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

    match clients[&token].try_clone()
    {
        Ok(mut stream) =>
        {
            loop
            {
                match Msg::from_stream(&mut stream)
                {
                    Ok(msg) =>
                    {
                        match &msg.command
                        {
                            b"addp        " => {
                                rcv_addp(
                                    &msg.payload,
                                    server,
                                    peers,
                                    db);
                            }
                            b"remp        " => {
                                rcv_remp(
                                    &msg.payload,
                                    peers);
                            }
                            b"lisp        " => {
                                rcv_lisp(
                                    &msg.payload,
                                    peers);
                            }
                            b"blnc        " => {
                                print!(" balance\n");
                            }
                            b"addt        " => {
                                print!(" add transaction\n");

                                rcv_addt(
                                    &msg.payload,
                                    db);
                            }
                            b"vdlt        " => {
                                print!(" validate transaction\n");
                            }
                            b"pent        " => {
                                print!(" pending transactions\n");
                            }
                            b"lisb        " => {
                                print!(" get blocks\n");
                            }
                            b"getb        " => {
                                print!(" get block\n");
                            }
                            b"addb        " => {
                                print!(" add block\n");
                            }
                            b"geth        " => {
                                print!(" get block height\n");
                            }
                            b"getl        " => {
                                print!(" get latest block\n");
                            }
                            b"chat        " => {
                                print!(" chat\n");
                            }
                            b"echo        " => {
                                print!(" echo\n");
                            }
                            b"resp        " => {
                                match &msg.payload[..12]
                                {
                                    b"lisp        " =>
                                    {
                                        rcv_resp_lisp(
                                            &msg.payload[12..],
                                            server,
                                            peers,
                                            db);
                                    }
                                    _ => {}
                                }
                            }
                            _ => {
                                print!("Unknown cmd: {}\n", String::from_utf8(msg.command.to_vec()).unwrap());
                            }
                        }
                    }
                    Err(e) =>
                    {
                        println!("Error reading stream: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => { }
    }
}

fn rcv_addp(
    payload: &[u8],
    server: &TcpListener,
    peers: &mut Vec<Peer>,
    db: &Connection)
{
    println!("rcv_addp {}", String::from_utf8(payload.to_vec()).unwrap());

    let cmpts: Vec<&[u8]> = payload.split({|x| *x == ':' as u8}).collect();
    if cmpts.len() == 2
    {
        let ip = String::from_utf8(cmpts[0].to_vec()).unwrap();
        let port = String::from_utf8(cmpts[1].to_vec()).unwrap().parse::<i32>().unwrap();
        let timestamp = UTC::now().timestamp();

        if  ip != server.local_addr().unwrap().ip().to_string() ||
            port != server.local_addr().unwrap().port() as i32
        {
            match net::TcpStream::connect((ip.as_str(), port as u16))
            {
                Ok(mut stream) =>
                {
                    let trans = db.transaction().unwrap();
                    trans.execute("LOCK TABLE peers IN SHARE ROW EXCLUSIVE MODE;", &[]).unwrap();
                    trans.execute(
                        "WITH upsert AS
                        (UPDATE peers SET timestamp = $3 WHERE ip = $1 AND port = $2 RETURNING *)
                        INSERT INTO peers (ip, port, timestamp) SELECT $1, $2, $3
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
}

fn rcv_remp(
    payload: &[u8],
    peers: &mut Vec<Peer>)
{
    println!("rcv_remp {}", String::from_utf8(payload.to_vec()).unwrap());

    let cmpts: Vec<&[u8]> = payload.split({|x| *x == ':' as u8}).collect();
    if cmpts.len() == 2
    {
        let ip = String::from_utf8(cmpts[0].to_vec()).unwrap();
        let port = String::from_utf8(cmpts[1].to_vec()).unwrap().parse::<i32>().unwrap();

        match peers.iter_mut().position(|p| p.addr == ip && p.port == port)
        {
            Some(peer_idx) =>
            {
                peers.remove(peer_idx);
            }
            None => {}
        }
    }
}

fn rcv_lisp(
    payload: &[u8],
    peers: &mut Vec<Peer>)
{
    println!("rcv_lisp {}", String::from_utf8(payload.to_vec()).unwrap());

    let cmpts: Vec<&[u8]> = payload.split({|x| *x == ':' as u8}).collect();
    if cmpts.len() == 2
    {
        let ip = String::from_utf8(cmpts[0].to_vec()).unwrap();
        let port = String::from_utf8(cmpts[1].to_vec()).unwrap().parse::<i32>().unwrap();

        match peers.iter_mut().position(|p| p.addr == ip && p.port == port)
        {
            Some(peer_idx) =>
            {
                let mut stream = peers[peer_idx].clone().socket.unwrap();

                let msg = Msg::new_list_peers_response(peers).to_vec();
                match stream.write(&msg)
                {
                    Ok(nbytes) => {
                        println!("resp_lisp bytes written: {} at {}", nbytes, UTC::now().time());
                    }
                    Err(e) => {
                        println!("Error writing to stream: {}", e);
                    }
                }
            }
            None => {}
        }
    }
}

fn rcv_resp_lisp(
    payload: &[u8],
    server: &TcpListener,
    peers: &mut Vec<Peer>,
    db: &Connection)
{
    println!("rcv_resp_lisp {}", String::from_utf8(payload.to_vec()).unwrap());

    let cmpts: Vec<&[u8]> = payload.split({|x| *x == ',' as u8}).collect();
    for addr in cmpts
    {
        rcv_addp(
            addr,
            server,
            peers,
            db);
    }
}

pub fn rcv_addt(
    payload: &[u8],
    db: &Connection)
{
    let tx = Tx::from_slice(payload);

    db.execute("BEGIN WORK;", &[]).unwrap();
    db.execute("LOCK TABLE transactions IN SHARE ROW EXCLUSIVE MODE;", &[]).unwrap();
    if db.execute(
        "SELECT EXISTS (SELECT 1 FROM transactions WHERE hash = $1)",
        &[&tx.hash.as_ref()])
        .unwrap() == 1
    {
        db.execute(
            "INSERT INTO transactions (hash, timestamp) SELECT $1, $2",
            &[&tx.hash.as_ref(), &tx.timestamp])
            .unwrap();
        for txi in tx.inputs.iter()
        {
            db.execute(
                "INSERT INTO tx_inputs (src_hash, src_idx, signature, tx) SELECT $1, $2, $3, (SELECT id FROM transactions WHERE hash = $4)",
                &[&txi.src_hash.as_ref(), &txi.src_idx, &txi.signature.as_ref(), &tx.hash.as_ref()])
                .unwrap();
        }
        for txo in tx.outputs.iter()
        {
            db.execute(
                "INSERT INTO tx_outputs (amount, address, tx) SELECT $1, $2, (SELECT id FROM transactions WHERE hash = $3)",
                &[&txo.amount, &txo.address.as_ref(), &tx.hash.as_ref()])
                .unwrap();
        }
    }
    db.execute("COMMIT WORK;", &[]).unwrap();
}
