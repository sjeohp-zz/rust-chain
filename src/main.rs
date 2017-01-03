pub mod block;
pub mod wallet;
pub mod transaction;
pub mod message;
pub mod peer;
mod util;
mod network;
mod mining;

use peer::*;
use wallet::*;
use transaction::*;
use message::*;
use block::*;
use util::*;
use network::*;
use mining::*;

use std::env;

extern crate mio;
use self::mio::channel::{channel};

extern crate rustyline;
use self::rustyline::error::ReadlineError;
use self::rustyline::Editor;

use std::thread;

extern crate postgres;
use self::postgres::{Connection, TlsMode};

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
                quit_snd.send(());
                network_child.join();
                break
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                quit_snd.send(());
                network_child.join();
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

#[cfg(test)]
mod tests {

    use message::*;
    use wallet::*;
    use transaction::*;
    use block::*;
    use util::*;
    use network::*;
    use std::env;

    extern crate mio;
    use self::mio::channel::{channel};

    extern crate rustyline;
    use self::rustyline::error::ReadlineError;
    use self::rustyline::Editor;

    use std::thread;

    #[test]
    fn test_new_transaction()
    {
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
        let tx1 = Tx::from_slice(&tx0.to_vec());
        assert!(tx0 == tx1);
    }

    #[test]
    #[ignore]
    fn test_block()
    {
        let tx = Tx::new(
            vec![],
            vec![
                Txo {
                    amount: 55555555,
                    address: [1; 32]
                }
            ],
            9000);
        let block = Block::new(
            vec![
                tx
            ],
            [2; 32],
            [0; 32],
            9001,
            0
        );
        println!("block hash: {:?}", to_hex_string(&block.block_hash));
        println!("txs hash: {:?}", to_hex_string(&block.txs_hash));
    }

    #[test]
    #[ignore]
    fn test_network()
    {
        let (quit, quit_rcv) = channel::<()>();
        let network_child = thread::spawn(move || {
            start_server(env::args().nth(1), quit_rcv);
        });
    }
}
