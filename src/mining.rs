extern crate mio;
use self::mio::channel::{channel, Receiver};

extern crate postgres;
use self::postgres::{Connection, TlsMode};
use self::postgres::rows::RowIndex;

extern crate rand;
use self::rand::{thread_rng, Rng};

extern crate num;
use self::num::bigint::{BigUint, ToBigUint};

use transaction::*;
use block::*;
use util::*;

const target_freq: i64 = 120;

pub fn start_mining(
    transaction_rcv: Receiver<Tx>,
    block_rcv: Receiver<Block>)
{
    let db_url = "postgresql://chain@localhost:5432/chaindb";
    let db = Connection::connect(db_url, TlsMode::None).expect("Unable to connect to database");

    println!("0");
    let blockchain: Vec<Block> = db.query(
        "SELECT txs_hash, parent_hash, target, timestamp, nonce, block_hash FROM blocks ORDER BY timestamp DESC;",
        &[])
        .unwrap()
        .iter()
        .map(|row|
            Block::new(
                vec![],
                &(row.get::<usize, Vec<u8>>(0)),
                &(row.get::<usize, Vec<u8>>(1)),
                &(row.get::<usize, Vec<u8>>(2)),
                row.get(3),
                row.get(4),
                &(row.get::<usize, Vec<u8>>(5))
            ))
        .collect();

    println!("1");
    let pending_txs: Vec<Tx> = db.query(
        "SELECT hash, timestamp, block FROM transactions WHERE block NOT IN (SELECT id FROM blocks);",
        &[])
        .unwrap()
        .iter()
        .map(|row|
            Tx::new_with_hash(
                &(row.get::<usize, Vec<u8>>(0)),
                row.get(1),
            ))
        .collect();

    println!("2");
    let mut target = [0; 32];
    let mut parent_hash = [0; 32];
    if blockchain.len() == 0
    {
        target[1] = u8::max_value();
    }
    else if blockchain.len() < 10
    {
        let mut dt = 0;
        for i in 0..blockchain.len()
        {
            dt += blockchain[i].timestamp - blockchain[i+1].timestamp;
        }
        dt /= 10;

        let newtarget = BigUint::from_bytes_le(&blockchain.first().unwrap().target) * dt.to_biguint().unwrap() / target_freq.to_biguint().unwrap();
        target.clone_from_slice(&newtarget.to_bytes_le());

        parent_hash.clone_from_slice(&blockchain.first().unwrap().block_hash);
    }
    else
    {
        let mut dt = 0;
        for i in 0..10
        {
            dt += blockchain[i].timestamp - blockchain[i+1].timestamp;
        }
        dt /= 10;

        let newtarget = BigUint::from_bytes_le(&blockchain.first().unwrap().target) * dt.to_biguint().unwrap() / target_freq.to_biguint().unwrap();
        target.clone_from_slice(&newtarget.to_bytes_le());

        parent_hash.clone_from_slice(&blockchain.first().unwrap().block_hash);
    }

    let mut rng = rand::thread_rng();
    let nonce = rng.gen::<i64>();

    let mut next_block = Block::new_minable(
        pending_txs,
        &parent_hash,
        &target,
        nonce);

    while !mine(&mut next_block) {}

    println!("{}", to_hex_string(&next_block.target));
    println!("{}", to_hex_string(&next_block.block_hash));
}
