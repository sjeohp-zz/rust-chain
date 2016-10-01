pub mod block;
pub mod transaction;

#[cfg(test)]
mod tests {

    use transaction::*;
    use block::*;

    #[test]
    fn test_transaction()
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

        println!("hash: {:?}", to_hex_string(&tx.hash));
    }

    #[test]
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
            [2; 32],
            [0; 32],
            9001,
            0,
            vec![
                tx
            ]
        );

        println!("block hash: {:?}", to_hex_string(&block.block_hash));
        println!("txs hash: {:?}", to_hex_string(&block.txs_hash));
    }
}
