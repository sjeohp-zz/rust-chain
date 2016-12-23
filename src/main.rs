pub mod block;
pub mod transaction;
mod util;
mod network;

use transaction::*;
use block::*;
use util::*;
use network::*;
use std::env;

pub fn main()
{
    start_server(env::args().nth(1));
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
