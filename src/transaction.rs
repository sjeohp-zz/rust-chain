extern crate ring;
extern crate untrusted;
extern crate byteorder;

use self::ring::{digest};

use self::byteorder::{ByteOrder, BigEndian};

use std::str;

const NBYTES_U64: usize = 8;

pub fn to_hex_string(bytes: Vec<u8>) -> String
{
  let strs: Vec<String> = bytes.iter().map(|b| format!("{:02x}", b)).collect();
  strs.join("")
}

pub struct Txi
{
    pub src_hash:   [u8; 32],
    pub src_idx:    u64,
    pub signature:  [u8; 32],
}

pub struct Txo
{
    pub amount:     u64,
    pub address:    [u8; 32],
}

pub struct Tx
{
    pub inputs:     Vec<Txi>,
    pub outputs:    Vec<Txo>,
    pub timestamp:  u64,
    pub hash:       [u8; 32],
}

pub fn tx_hash(tx: Tx) -> Vec<u8>
{
    let mut txi_buf: Vec<u8> = vec![];
    for x in tx.inputs
    {
        let mut buf = [0; NBYTES_U64];
        BigEndian::write_u64(&mut buf, x.src_idx);
        txi_buf.extend_from_slice(&x.src_hash);
        txi_buf.extend_from_slice(&buf);
        txi_buf.extend_from_slice(&x.signature);
    }

    let mut txo_buf: Vec<u8> = vec![];
    for x in tx.outputs
    {
        let mut buf = [0; NBYTES_U64];
        BigEndian::write_u64(&mut buf, x.amount);
        txo_buf.extend_from_slice(&buf);
        txo_buf.extend_from_slice(&x.address);
    }

    let mut tms_buf = [0; NBYTES_U64];
    BigEndian::write_u64(&mut tms_buf, tx.timestamp);

    let mut txn_buf = vec![];
    txn_buf.extend_from_slice(&txi_buf);
    txn_buf.extend_from_slice(&txo_buf);
    txn_buf.extend_from_slice(&mut tms_buf);

    digest::digest(&digest::SHA256, &txn_buf).as_ref().to_vec()
}
