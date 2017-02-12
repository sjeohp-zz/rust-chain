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
        txs_hash: &[u8],
        txs: Vec<Tx>,
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

    pub fn from_slice(bytes: &[u8]) -> Block
    {
        let txs_hash_len = 32;
        let ntxs_len = 4;
        let tx_len_len = 4;
        let parent_hash_len = 32;
        let target_len = 32;
        let timestamp_len = 8;
        let nonce_len = 8;
        let block_hash_len = 32;

        let mut idx: usize = 0;

        let txs_hash = &bytes[..txs_hash_len];
        idx += txs_hash_len;

        let ntxs = LittleEndian::read_u32(&bytes[idx..idx+ntxs_len]) as usize;
        idx += ntxs_len;

        let mut txs = vec![];
        for _ in 0..ntxs
        {
            let tx_len = LittleEndian::read_u32(&bytes[idx..idx+tx_len_len]) as usize;
            idx += tx_len_len;

            let tx = Tx::from_slice(&bytes[idx..idx+tx_len]);
            idx += tx_len;

            txs.push(tx);
        }

        let parent_hash = &bytes[idx..idx+parent_hash_len];
        idx += parent_hash_len;
        let target = &bytes[idx..idx+target_len];
        idx += target_len;
        let timestamp = LittleEndian::read_i64(&bytes[idx..idx+timestamp_len]);
        idx += timestamp_len;
        let nonce = LittleEndian::read_i64(&bytes[idx..idx+nonce_len]);
        idx += nonce_len;
        let block_hash = &bytes[idx..idx+block_hash_len];

        Block::new(
            txs_hash,
            txs,
            parent_hash,
            target,
            timestamp,
            nonce,
            block_hash)
    }

    pub fn to_vec(&self) -> Vec<u8>
    {
        let mut array = vec![];
        array.extend_from_slice(&self.txs_hash);

        let mut ntxs_buf = [0; 4];
        LittleEndian::write_u32(&mut ntxs_buf, self.txs.len() as u32);
        array.extend_from_slice(&ntxs_buf);

        for tx in self.txs.iter()
        {
            let tx_vec = tx.to_vec();
            let tx_len = [0; 4];
            LittleEndian::write_u32(&mut ntxs_buf, tx_vec.len() as u32);
            array.extend_from_slice(&tx_len);
            array.extend_from_slice(&tx_vec);
        }

        array.extend_from_slice(&self.parent_hash);

        array.extend_from_slice(&self.target);

        let mut ts_buf = [0; 8];
        LittleEndian::write_i64(&mut ts_buf, self.timestamp as i64);
        array.extend_from_slice(&ts_buf);

        let mut nonce_buf = [0; 8];
        LittleEndian::write_i64(&mut nonce_buf, self.nonce as i64);
        array.extend_from_slice(&nonce_buf);

        array.extend_from_slice(&self.block_hash);

        array
    }

    pub fn verify(&mut self) -> bool
    {
        let mut tx_verify = true;
        for tx in self.txs.iter_mut()
        {
            if !tx.verify() { tx_verify = false; }
        }
        tx_verify && block_hash(self) == self.block_hash.to_vec()
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
    if block.nonce == i64::max_value() { block.nonce = 0; } else { block.nonce += 1; }
    let hash = block_hash(block);
    block.block_hash.clone_from_slice(&hash);
    block.block_hash < block.target
}
