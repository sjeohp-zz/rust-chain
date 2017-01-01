extern crate ring;
extern crate untrusted;
extern crate byteorder;
extern crate chrono;

use self::ring::{digest};

use self::byteorder::{ByteOrder, LittleEndian};

use transaction::*;
use util::{NBYTES_U64};

use self::chrono::*;

pub struct Block
{
    pub txs:            Vec<Tx>,
    pub txs_hash:       [u8; 32],
    pub parent_hash:    [u8; 32],
    pub target:         [u8; 32],
    pub timestamp:      i64,
    pub nonce:          i64,
    pub block_hash:     [u8; 32],
}

impl Block
{
    pub fn new(
        txs: Vec<Tx>,
        txs_hash: &[u8],
        parent_hash: &[u8],
        target: &[u8],
        timestamp: i64,
        nonce: i64,
        block_hash: &[u8]) -> Block
    {
        let mut block = Block {
            txs_hash: [0; 32],
            txs: txs,
            parent_hash: [0; 32],
            target: [0; 32],
            timestamp: timestamp,
            nonce: nonce,
            block_hash: [0; 32]
        };
        block.txs_hash.clone_from_slice(txs_hash);
        block.parent_hash.clone_from_slice(parent_hash);
        block.target.clone_from_slice(target);
        block.block_hash.clone_from_slice(block_hash);
        block
    }

    pub fn new_minable(
        txs: Vec<Tx>,
        parent_hash: &[u8],
        target: &[u8],
        nonce: i64) -> Block
    {
        let mut block = Block {
            txs_hash: [0; 32],
            txs: txs,
            parent_hash: [0; 32],
            target: [0; 32],
            timestamp: 0,
            nonce: nonce,
            block_hash: [0; 32]
        };
        block.txs_hash.clone_from_slice(&txs_hash(&block.txs));
        block.parent_hash.clone_from_slice(parent_hash);
        block.target.clone_from_slice(target);
        block
    }
}

pub fn txs_hash(txs: &[Tx]) -> Vec<u8>
{
    let txs_hash_bytes: Vec<u8> = txs.iter().flat_map(|x| x.hash.to_vec()).collect();
    digest::digest(&digest::SHA256, &txs_hash_bytes).as_ref().to_vec()
}

pub fn block_hash(block: &Block) -> Vec<u8>
{
    let mut block_buf: Vec<u8> = vec![];
    block_buf.extend_from_slice(&block.txs_hash);
    block_buf.extend_from_slice(&block.parent_hash);
    block_buf.extend_from_slice(&block.target);
    let mut tms_buf = [0; NBYTES_U64];
    LittleEndian::write_i64(&mut tms_buf, block.timestamp);
    block_buf.extend_from_slice(&tms_buf);
    let mut nonce_buf = [0; NBYTES_U64];
    LittleEndian::write_i64(&mut nonce_buf, block.nonce);
    block_buf.extend_from_slice(&nonce_buf);
    digest::digest(&digest::SHA256, &block_buf).as_ref().to_vec()
}

pub fn mine(block: &mut Block) -> bool
{
    block.timestamp = UTC::now().timestamp();
    block.nonce += 1;
    let hash = block_hash(block);
    block.block_hash.clone_from_slice(&hash);
    block.block_hash < block.target
}
