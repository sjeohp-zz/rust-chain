pub mod block;
pub mod transaction;

#[cfg(test)]
mod tests {

    use transaction::*;

    #[test]
    fn test_transaction()
    {
        let mut tx = Tx {
            inputs: vec![],
            outputs: vec![
                Txo {
                    amount: 55555555,
                    address: [1; 32]
                }
            ],
            timestamp: 9000,
            hash: [0; 32]
        };

        let hash = tx_hash(tx);

        println!("hash: {:?}", to_hex_string(hash));
    }
}
