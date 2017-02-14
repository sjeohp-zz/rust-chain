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
use database;

const TARGET_FREQ: i64 = 10;

pub fn start_mining(
    transaction_rcv_from_network: Receiver<Transaction>,
    block_rcv_from_network: Receiver<Block>,
    block_snd_to_network: Sender<Block>)
{
    let db = database::conn();

    'outer: loop
    {
        let blockchain: Vec<Block> = database::blockchain(&db);
        let mut pending_txs: Vec<Transaction> = database::pending_txs(&db);

        for tx in pending_txs.iter_mut()
        {
            tx.inputs = database::tx_inputs(&tx, &db);
            tx.outputs = database::tx_outputs(&tx, &db);
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

        database::insert_block(&next_block, &db);

        let _ = block_snd_to_network.send(next_block);
    }
}
