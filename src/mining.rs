extern crate mio;
use self::mio::channel::{Sender, Receiver};

extern crate postgres;
use self::postgres::{Connection, TlsMode};

extern crate rand;
use self::rand::{Rng};

extern crate num;
use self::num::bigint::{BigUint, ToBigUint};

use transaction::*;
use block::*;
use util::*;

const TARGET_FREQ: i64 = 10;

pub fn start_mining(
    transaction_rcv_from_network: Receiver<Tx>,
    block_rcv_from_network: Receiver<Block>,
    block_snd_to_network: Sender<Block>)
{
    let db_url = "postgresql://chain@localhost:5432/chaindb";
    let db = Connection::connect(db_url, TlsMode::None).expect("Unable to connect to database");

    'outer: loop
    {
        let blockchain: Vec<Block> = db.query(
            "SELECT txs_hash, parent_hash, target, timestamp, nonce, block_hash FROM blocks ORDER BY timestamp ASC;",
            &[])
            .unwrap()
            .iter()
            .map(|row|
                Block::new(
                    &(row.get::<usize, Vec<u8>>(0)),
                    vec![],
                    &(row.get::<usize, Vec<u8>>(1)),
                    &(row.get::<usize, Vec<u8>>(2)),
                    row.get(3),
                    row.get(4),
                    &(row.get::<usize, Vec<u8>>(5))
                ))
            .collect();

        let mut pending_txs: Vec<Tx> = db.query(
            "SELECT hash, timestamp, block FROM transactions WHERE block NOT IN (SELECT block_hash FROM blocks);",
            &[])
            .unwrap()
            .iter()
            .map(|row|
                Tx::new_with_hash(
                    &(row.get::<usize, Vec<u8>>(0)),
                    row.get(1),
                ))
            .collect();

        for tx in pending_txs.iter_mut()
        {
            let inputs: Vec<Txi> = db.query(
                "SELECT src_hash, src_idx, signature FROM tx_inputs WHERE tx = $1;",
                &[&tx.hash.as_ref()])
                .unwrap()
                .iter()
                .map(|row|
                    Txi::from_stored(
                        &(row.get::<usize, Vec<u8>>(0)),
                        row.get(1),
                        &(row.get::<usize, Vec<u8>>(2))
                    ))
                .collect();
            tx.inputs = inputs;

            let outputs: Vec<Txo> = db.query(
                "SELECT amount, address FROM tx_outputs WHERE tx = $1;",
                &[&tx.hash.as_ref()])
                .unwrap()
                .iter()
                .map(|row|
                    Txo::new(
                        row.get(0),
                        &(row.get::<usize, Vec<u8>>(1))
                    ))
                .collect();
            tx.outputs = outputs;
        }

        let mut target = [0; 32];
        let mut parent_hash = [0; 32];
        if blockchain.len() < 2
        {
            target[2] = u8::max_value()/2;
        }
        else
        {
            let n = if blockchain.len() < 10 { blockchain.len() } else { 10 };
            let mut dt = 0;
            let mut sumtarget = 0.to_biguint().unwrap();
            let mut count: i64 = 0;
            for i in (blockchain.len()-n..blockchain.len()-1).rev()
            {
                count += 1;
                dt += blockchain[i+1].timestamp - blockchain[i].timestamp;
                println!("{} {}", blockchain[i].timestamp, blockchain[i+1].timestamp);
                sumtarget = sumtarget + BigUint::from_bytes_be(&blockchain[i+1].target);
            }
            dt /= count;
            sumtarget = sumtarget / count.to_biguint().unwrap();

            println!("count: {}", count);
            println!("sumtarget: {}", sumtarget);

            println!("oldtarget: {}", to_hex_string(&blockchain.last().unwrap().target));
            println!("oldtarget: {}", BigUint::from_bytes_be(&blockchain.last().unwrap().target));
            println!("dt {}", dt);

            if dt > 0
            {
                let newtarget = sumtarget * dt.to_biguint().unwrap() / TARGET_FREQ.to_biguint().unwrap();
                let bytes = newtarget.to_bytes_be();
                target[32-bytes.len()..].clone_from_slice(&bytes);
            }
            else
            {
                target.clone_from_slice(&blockchain.last().unwrap().target);
            }

            println!("newtarget: {}", to_hex_string(&target));
            println!("newtarget: {}", BigUint::from_bytes_be(&target));

            parent_hash.clone_from_slice(&blockchain.last().unwrap().block_hash);
        }

        let mut rng = rand::thread_rng();
        let nonce = rng.gen::<i64>();

        let mut next_block = Block::new_minable(
            pending_txs,
            &parent_hash,
            &target,
            nonce);

        'inner: while !mine(&mut next_block)
        {
            match transaction_rcv_from_network.try_recv()
            {
                Ok(_) => {
                    continue 'outer;
                }
                Err(_) => {}
            }
            match block_rcv_from_network.try_recv()
            {
                Ok(_) => {
                    continue 'outer;
                }
                Err(_) => {}
            }
        }

        println!("{:#?}", &next_block.to_vec());

        println!("{}", to_hex_string(&next_block.target));
        println!("{}", to_hex_string(&next_block.block_hash));

        db.execute("BEGIN WORK;", &[]).unwrap();
        db.execute("LOCK TABLE blocks IN SHARE ROW EXCLUSIVE MODE;", &[]).unwrap();
        db.execute(
            "INSERT INTO blocks (txs_hash, parent_hash, target, timestamp, nonce, block_hash) SELECT $1, $2, $3, $4, $5, $6",
            &[&next_block.txs_hash.as_ref(), &next_block.parent_hash.as_ref(), &next_block.target.as_ref(), &next_block.timestamp, &next_block.nonce, &next_block.block_hash.as_ref()])
            .unwrap();
        for tx in next_block.txs.iter()
        {
            db.execute(
                "UPDATE transactions SET block = $1 WHERE hash = $2",
                &[&next_block.block_hash.as_ref(), &tx.hash.as_ref()])
                .unwrap();
        }
        db.execute("COMMIT WORK;", &[]).unwrap();

        let _ = block_snd_to_network.send(next_block);
    }
}
