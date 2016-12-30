extern crate ring;
extern crate untrusted;
extern crate byteorder;

use self::ring::{digest};

use self::byteorder::{ByteOrder, LittleEndian};

use transaction::*;
use util::{NBYTES_U64};

pub struct Block
{
    pub txs:            Vec<Tx>,
    pub txs_hash:       [u8; 32],
    pub parent_hash:    [u8; 32],
    pub target:         [u8; 32],
    pub timestamp:      u64,
    pub nonce:          u64,
    pub block_hash:     [u8; 32],
}

impl Block
{
    pub fn new(
        txs: Vec<Tx>,
        parent_hash: [u8; 32],
        target: [u8; 32],
        timestamp: u64,
        nonce: u64
    ) -> Block
    {
        let mut block = Block {
            txs_hash: [0; 32],
            txs: txs,
            parent_hash: parent_hash,
            target: target,
            timestamp: timestamp,
            nonce: nonce,
            block_hash: [0; 32]
        };
        let txs_hash = block.gen_txs_hash();
        for i in 0..32 { block.txs_hash[i] = txs_hash[i]; }
        let block_hash = block.gen_block_hash();
        for i in 0..32 { block.block_hash[i] = block_hash[i]; }
        block
    }

    pub fn gen_txs_hash(&mut self) -> Vec<u8>
    {
        let txs: Vec<u8> = self.txs.iter().flat_map(|x| x.hash.to_vec()).collect();
        digest::digest(&digest::SHA256, &txs).as_ref().to_vec()
    }

    pub fn gen_block_hash(&mut self) -> Vec<u8>
    {
        let mut block_buf: Vec<u8> = vec![];
        block_buf.extend_from_slice(&self.txs_hash);
        block_buf.extend_from_slice(&self.parent_hash);
        block_buf.extend_from_slice(&self.target);
        let mut tms_buf = [0; NBYTES_U64];
        LittleEndian::write_u64(&mut tms_buf, self.timestamp);
        block_buf.extend_from_slice(&tms_buf);
        let mut nonce_buf = [0; NBYTES_U64];
        LittleEndian::write_u64(&mut nonce_buf, self.nonce);
        block_buf.extend_from_slice(&nonce_buf);
        digest::digest(&digest::SHA256, &block_buf).as_ref().to_vec()
    }
}
