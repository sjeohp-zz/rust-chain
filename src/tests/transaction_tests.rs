use transaction::*;

#[test]
fn test_new_transaction()
{
    let mut tx0 = Tx::new(
        vec![
            Txi {
                src_hash:   [1; 32],
                src_idx:    2,
                signature:  [3; 64]
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
