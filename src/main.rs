pub mod block;
pub mod wallet;
pub mod transaction;
pub mod message;
pub mod peer;
mod util;
mod network;
mod mining;
mod tests;

use transaction::*;
use block::*;
use network::*;
use mining::*;

use std::env;

extern crate mio;
use self::mio::channel::{channel};

extern crate rustyline;
use self::rustyline::error::ReadlineError;
use self::rustyline::Editor;

use std::thread;

pub fn main()
{
    let (transaction_snd_to_mine, transaction_rcv_from_network) = channel::<Tx>();
    let (block_snd_to_mine, block_rcv_from_network) = channel::<Block>();
    let (block_snd_to_network, block_rcv_from_mine) = channel::<Block>();
    let mining_child = thread::spawn(move || {
        start_mining(
            transaction_rcv_from_network,
            block_rcv_from_network,
            block_snd_to_network);
    });

    let (quit_snd, quit_rcv) = channel::<()>();
    let network_child = thread::spawn(move || {
        start_server(
            env::args().nth(1),
            quit_rcv,
            transaction_snd_to_mine,
            block_snd_to_mine,
            block_rcv_from_mine);
    });

    let mut rl = Editor::<()>::new();
    if let Err(_) = rl.load_history("history.txt") {
        println!("No previous history.");
    }
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(&line);
                println!("Line: {}", line);
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                let _ = quit_snd.send(());
                let _ = network_child.join();
                let _ = mining_child.join();
                break
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                let _ = quit_snd.send(());
                let _ = network_child.join();
                let _ = mining_child.join();
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
    }
    rl.save_history("history.txt").unwrap();
}
