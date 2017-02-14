use transaction::*;
use block::*;
use mining;
use wallet;
use crypto;
use database;

extern crate chrono;
use self::chrono::{UTC};

#[test]
fn test_transactions()
{
    let db = database::conn();

    let mut public_key: [u8; 32] = [0; 32];
    let mut private_key: [u8; 32] = [0; 32];
    wallet::get_keypair(&mut public_key, &mut private_key);

    let mut other_public_key: [u8; 32] = [0; 32];
    let mut other_private_key: [u8; 32] = [0; 32];
    crypto::gen_ed25519keypair(&mut other_public_key, &mut other_private_key);

    let tx0_inp = vec![];
    let tx0_out = vec![
        TxOutput::new(42, &public_key)
    ];
    let ts0 = UTC::now().timestamp();
    let tx0 = Transaction::new(
        tx0_inp,
        tx0_out,
        ts0
    );

    let tx1_inp = vec![
        TxInput::new(&tx0.hash, 0)
    ];
    let tx1_out = vec![
        TxOutput::new(21, &other_public_key),
        TxOutput::new(21, &public_key)
    ];
    let ts1 = UTC::now().timestamp();
    let tx1 = Transaction::new(
        tx1_inp,
        tx1_out,
        ts1
    );

    assert!(tx0 == Transaction::from_slice(&tx0.to_vec()));
    assert!(tx1 == Transaction::from_slice(&tx1.to_vec()));

    let mut block = Block::new_minable(
        vec![tx0, tx1],
        &[0; 32],
        &[<u8>::max_value(); 32], // MAXIMUM TARGET == ZERO MINING DIFFICULTY
        0);

    assert!(mining::mine(&mut block)); // FAILING TO MINE THIS BLOCK SHOULD BE IMPOSSIBLE
    assert!(database::insert_block(&block, &db).is_ok());
    assert!(wallet::balance() == 21);
}
