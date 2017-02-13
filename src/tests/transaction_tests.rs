use transaction::*;
use wallet;
use crypto;

extern crate chrono;
use self::chrono::{UTC};

#[test]
fn test_transactions()
{
    let mut public_key: [u8; 32] = [0; 32];
    let mut private_key: [u8; 32] = [0; 32];
    wallet::get_keypair(&mut public_key, &mut private_key);

    let mut other_public_key: [u8; 32] = [0; 32];
    let mut other_private_key: [u8; 32] = [0; 32];
    crypto::gen_ed25519keypair(&mut public_key, &mut private_key);

    let tx0_inp = vec![];
    let tx0_out = vec![
        Txo::new(42, &public_key)
    ];
    let ts0 = UTC::now().timestamp();
    let tx0 = Tx::new(
        tx0_inp,
        tx0_out,
        ts0
    );

    let tx1_inp = vec![
        Txi::new(&tx0.hash, 0)
    ];
    let tx1_out = vec![
        Txo::new(21, &other_public_key),
        Txo::new(21, &public_key)
    ];
    let ts1 = UTC::now().timestamp();
    let tx1 = Tx::new(
        tx1_inp,
        tx1_out,
        ts1
    );

    assert!(tx0 == Tx::from_slice(&tx0.to_vec()));
    assert!(tx1 == Tx::from_slice(&tx1.to_vec()))
}
