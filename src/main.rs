pub mod block;
pub mod transaction;
mod util;
mod network;

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

pub fn main()
{
    let (quit, quit_rcv) = channel::<()>();
    let network_child = thread::spawn(move || {
        start_server(env::args().nth(1), quit_rcv);
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
                quit.send(());
                network_child.join();
                break
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                quit.send(());
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

    use transaction::*;
    use block::*;
    use util::*;
    use network::*;
    use std::env;

    #[test]
    #[ignore]
    fn test_new_transaction()
    {
        let tx = Tx::new(
            vec![],
            vec![
                Txo {
                    amount: 0,
                    address: [0; 32]
                }
            ],
            0
        );
        println!("hash: {:?}", to_hex_string(&tx.hash));
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
    fn test_network()
    {
        start_server(env::args().nth(0));
    }
}
