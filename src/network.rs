use database;
use transaction::*;
use peer::*;
use message::*;
use block::*;
use wallet;

extern crate mio;
extern crate chrono;
extern crate postgres;
extern crate rand;

extern crate byteorder;
use self::byteorder::{ByteOrder, LittleEndian};

// use util::{NBYTES_U64, NBYTES_U32};

use self::mio::*;
use self::mio::channel::{Sender, Receiver};
use self::mio::tcp::{TcpListener, TcpStream};

use self::chrono::*;

use self::postgres::{Connection};

use std::collections::{HashMap};
// use std::time;
use std::io::{Write};
use std::net;

// use std::thread;
// use self::rand::{thread_rng, Rng};

const QUIT_TOKEN: Token = Token(0);
const MINED_BLOCK_TOKEN: Token = Token(1);
const SERVER_TOKEN: Token = Token(2);
const CLIENT_TOKEN_COUNTER: usize = 3;
const LOCALHOST: &'static str = "127.0.0.1";
const SERVER_DEFAULT_PORT: &'static str = "9001";

pub fn start_server(
    port: Option<String>,
    quit_rcv: Receiver<()>,
    transaction_snd_to_mine: Sender<Transaction>,
    block_snd_to_mine: Sender<Block>,
    block_rcv_from_mine: Receiver<Block>)
{
    let db = database::conn();

    let poll = Poll::new().expect("Failed to create poll");

    poll.register(
        &quit_rcv,
        QUIT_TOKEN,
        Ready::readable(),
        PollOpt::level()).expect("Failed to register UI receiver channel");

    poll.register(
        &block_rcv_from_mine,
        MINED_BLOCK_TOKEN,
        Ready::readable(),
        PollOpt::level()).expect("Failed to register block receiver channel");

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

    let peer_history = database::peers(&db);

    let mut peers = bootstrap(LOCALHOST, &port, peer_history);

    for peer in &peers
    {
        let _ = database::upsert_peer(&peer, &db);
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

                MINED_BLOCK_TOKEN => {
                    println!("block received from mine");
                    let block = block_rcv_from_mine.try_recv().unwrap();
                    publish_block(block, &peers);
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
                        &db,
                        &transaction_snd_to_mine,
                        &block_snd_to_mine,
                    );
                }
            }
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
            let addr = peer.ip.clone() + ":" + &peer.port.to_string();
            println!("Connecting to peer at {}", addr);

            match net::TcpStream::connect((peer.ip.as_str(), peer.port as u16))
            {
                Ok(mut stream) => {
                    println!("Connected to {:?}", stream.peer_addr().unwrap());

                    let mut addp = Msg::new_add_peer(server_addr, server_port).to_vec();
                    let mut lisp = Msg::new_list_peers(server_addr, server_port).to_vec();

                    let tx0 = Transaction::new(
                        vec![
                            TxInput {
                                src_hash:   [1; 32],
                                src_idx:    2,
                                signature:  [3; 64]
                            }
                        ],
                        vec![
                            TxOutput {
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
                            println!("Bytes written: {}", nbytes);
                        }
                        Err(e) => {
                            println!("Error writing to stream: {}", e);
                        }
                    }
                    let _ = stream.flush();

                    peers.push(
                        Peer::new(
                            peer.ip,
                            peer.port,
                            UTC::now().timestamp(),
                            Some(stream)));
                }
                Err(e) => {
                    println!("Error connecting to host: {}", e);
                }
            }
        }
    }
    peers
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
                println!("Bytes written: {}", nbytes);
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
    db: &Connection,
    transaction_snd_to_mine: &Sender<Transaction>,
    block_snd_to_mine: &Sender<Block>)
{
    // let mut rng = rand::thread_rng();
    // let stutter = time::Duration::from_millis(rng.gen_range::<u64>(0, 5000));
    // thread::sleep(stutter);

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
                                rcv_blnc(
                                    &msg.payload,
                                    peers);
                            }
                            b"addt        " => {
                                rcv_addt(
                                    &msg.payload,
                                    db,
                                    transaction_snd_to_mine);
                            }
                            b"vdlt        " => {
                                print!(" validate transaction\n");
                                rcv_vldt(
                                    &msg.payload,
                                    peers);
                            }
                            b"pent        " => {
                                print!(" pending transactions\n");
                            }
                            b"lisb        " => {
                                print!(" get blocks\n");
                            }
                            b"getb        " => {
                                rcv_getb(
                                    &msg.payload,
                                    server,
                                    peers,
                                    db);
                            }
                            b"addb        " => {
                                rcv_addb(
                                    &msg.payload,
                                    db,
                                    block_snd_to_mine)
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
                                    b"blnc        " =>
                                    {
                                        rcv_resp_blnc(
                                            &msg.payload[12..]);
                                    }
                                    b"vldt        " =>
                                    {
                                        rcv_resp_vldt(
                                            &msg.payload[12..]);
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
        Err(_) => { }
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

        add_peer(ip, port, server, peers, db);
    }
}

fn add_peer(
    ip: String,
    port: i32,
    server: &TcpListener,
    peers: &mut Vec<Peer>,
    db: &Connection)
{
    let timestamp = UTC::now().timestamp();

    if  ip != server.local_addr().unwrap().ip().to_string() ||
        port != server.local_addr().unwrap().port() as i32
    {
        match net::TcpStream::connect((ip.as_str(), port as u16))
        {
            Ok(stream) =>
            {
                let peer = Peer::new(ip, port, timestamp, Some(stream));
                if database::upsert_peer(&peer, db).is_ok()
                {
                    peers.push(peer);
                }
            }
            Err(_) => {}
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

        match peers.iter_mut().position(|p| p.ip == ip && p.port == port)
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

        match peers.iter_mut().position(|p| p.ip == ip && p.port == port)
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

pub fn rcv_blnc(
    payload: &[u8],
    peers: &mut Vec<Peer>)
{
    let cmpts: Vec<&[u8]> = payload.split({|x| *x == ',' as u8}).collect();
    if cmpts.len() == 2
    {
        let balance = wallet::balance(cmpts[0]);
        let cmpts: Vec<&[u8]> = cmpts[1].split({|x| *x == ':' as u8}).collect();

        if cmpts.len() == 2
        {
            let ip = String::from_utf8(cmpts[0].to_vec()).unwrap();
            let port = String::from_utf8(cmpts[1].to_vec()).unwrap().parse::<i32>().unwrap();

            match peers.iter_mut().position(|p| p.ip == ip && p.port == port)
            {
                Some(peer_idx) =>
                {
                    let mut stream = peers[peer_idx].clone().socket.unwrap();

                    let msg = Msg::new_balance_response(balance).to_vec();
                    match stream.write(&msg)
                    {
                        Ok(nbytes) => {
                            println!("resp_blnc bytes written: {} at {}", nbytes, UTC::now().time());
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
}

pub fn rcv_resp_blnc(
    payload: &[u8])
{
    let balance = LittleEndian::read_i64(&payload);
    println!("Balance received: {}", balance);
}

pub fn rcv_addt(
    payload: &[u8],
    db: &Connection,
    transaction_snd_to_mine: &Sender<Transaction>)
{
    println!("rcv_addt");

    let mut tx = Transaction::from_slice(payload);
    if tx.verify()
    {
        if database::insert_transaction(&tx, db).is_ok()
        {
            let _ = transaction_snd_to_mine.send(tx);
        }
    }
    else
    {
        println!("Invalid transaction");
    }
}

pub fn rcv_vldt(
    payload: &[u8],
    peers: &mut Vec<Peer>)
{
    println!("rcv_vldt");

    let cmpts: Vec<&[u8]> = payload.split({|x| *x == ',' as u8}).collect();
    if cmpts.len() == 2
    {
        let mut tx = Transaction::from_slice(cmpts[0]);
        let valid = tx.verify();

        let cmpts: Vec<&[u8]> = cmpts[1].split({|x| *x == ':' as u8}).collect();
        if cmpts.len() == 2
        {
            let ip = String::from_utf8(cmpts[0].to_vec()).unwrap();
            let port = String::from_utf8(cmpts[1].to_vec()).unwrap().parse::<i32>().unwrap();

            match peers.iter_mut().position(|p| p.ip == ip && p.port == port)
            {
                Some(peer_idx) =>
                {
                    let mut stream = peers[peer_idx].clone().socket.unwrap();

                    let msg = Msg::new_validate_response(valid).to_vec();
                    match stream.write(&msg)
                    {
                        Ok(nbytes) => {
                            println!("resp_vldt bytes written: {} at {}", nbytes, UTC::now().time());
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
}

pub fn rcv_resp_vldt(
    payload: &[u8])
{
    let valid = LittleEndian::read_i32(&payload);
    println!("Valid received: {}", valid);
}

pub fn rcv_addb(
    payload: &[u8],
    db: &Connection,
    block_snd_to_mine: &Sender<Block>)
{
    println!("rcv_addb");
    let mut block = Block::from_slice(payload);
    if block.verify()
    {
        if database::insert_block(&block, db).is_ok()
        {
            let _ = block_snd_to_mine.send(block);
        }
    }
    else
    {
        println!("Invalid block");
    }
}

pub fn rcv_getb(
    payload: &[u8],
    server: &TcpListener,
    peers: &mut Vec<Peer>,
    db: &Connection)
{
    let payload_cmpts: Vec<&[u8]> = payload.split({|x| *x == ',' as u8}).collect();
    if payload_cmpts.len() > 1
    {
        let hash = payload_cmpts[0];
        let addr = payload_cmpts[1];
        let addr_cmpts: Vec<&[u8]> = addr.split({|x| *x == ':' as u8}).collect();
        if addr_cmpts.len() > 1
        {
            let ip = String::from_utf8(addr_cmpts[0].to_vec()).unwrap();
            let port = String::from_utf8(addr_cmpts[1].to_vec()).unwrap().parse::<i32>().unwrap();

            add_peer(ip.clone(), port, server, peers, db);

            let peer_idx = peers.iter_mut().position(|p| p.ip == ip && p.port == port).unwrap();

            match database::block(&hash, db)
            {
                Some(block) => {
                    let msg = Msg::new_add_block(block.to_vec()).to_vec();
                    let mut stream = peers[peer_idx].clone().socket.unwrap();
                    match stream.write(&msg)
                    {
                        Ok(nbytes) => {
                            println!("Bytes written: {}", nbytes);
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
}

pub fn publish_block(
    block: Block,
    peers: &[Peer])
{
    let msg = Msg::new_add_block(block.to_vec());
    for peer in peers.iter()
    {
        let p = peer.clone();
        let mut socket = p.socket.unwrap().try_clone().unwrap();
        println!("writing block msg {:#?}", &msg.to_vec());
        match socket.write(&msg.to_vec())
        {
            Ok(nbytes) => {
                println!("Bytes written: {}", nbytes);
            }
            Err(e) => {
                println!("Error writing to stream: {}", e);
            }
        }
        let _ = socket.flush();
    }
}
