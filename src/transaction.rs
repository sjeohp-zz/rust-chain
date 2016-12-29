extern crate ring;
extern crate untrusted;
extern crate byteorder;

use self::ring::{digest};

use self::byteorder::{ByteOrder, BigEndian};

use util::{NBYTES_U64, NBYTES_U32};

use std::mem::size_of;

#[derive(Clone, Debug, PartialEq)]
pub struct Txi
{
    pub src_hash:   [u8; 32],
    pub src_idx:    u64,
    pub signature:  [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Txo
{
    pub amount:     u64,
    pub address:    [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tx
{
    pub hash:       [u8; 32],
    pub timestamp:  u64,
    pub inputs:     Vec<Txi>,
    pub outputs:    Vec<Txo>,
}

impl Tx
{
    pub fn new(
        inputs: Vec<Txi>,
        outputs: Vec<Txo>,
        timestamp: u64
    ) -> Tx
    {
        let mut tx = Tx {
            hash: [0; 32],
            timestamp: timestamp,
            inputs: inputs,
            outputs: outputs,
        };
        let tx_hash = tx.gen_hash();
        for i in 0..32 { tx.hash[i] = tx_hash[i]; }
        tx
    }

    pub fn from_slice(bytes: &[u8]) -> Tx
    {
        let hash = &bytes[..32];
        let timestamp = BigEndian::read_u64(&bytes[32..40]);
        let mut idx: usize = 40;
        let inp_len = (BigEndian::read_u32(&bytes[idx..idx+4]) as usize) * size_of::<Txi>();
        idx += 4;
        let input_bytes = &bytes[idx..idx+(inp_len)];
        idx += inp_len;
        let out_len = (BigEndian::read_u32(&bytes[idx..idx+4]) as usize) * size_of::<Txo>();
        idx += 4;
        let output_bytes = &bytes[idx..idx+(out_len)];

        let mut inputs = vec![];
        for i in 0..(inp_len / size_of::<Txi>())
        {
            let mut txi = Txi {
                src_hash: [0; 32],
                src_idx: BigEndian::read_u64(&input_bytes[(i*size_of::<Txi>()+32)..(i*size_of::<Txi>()+40)]),
                signature: [0; 32]
            };
            txi.src_hash.clone_from_slice(&input_bytes[(i*size_of::<Txi>()) as usize..(i*size_of::<Txi>()+32) as usize]);
            txi.signature.clone_from_slice(&input_bytes[(i*size_of::<Txi>()+40) as usize..(i*size_of::<Txi>()+72) as usize]);
            inputs.push(txi);
        }

        let mut outputs = vec![];
        for i in 0..(out_len / size_of::<Txo>())
        {
            let mut txo = Txo {
                amount: BigEndian::read_u64(&output_bytes[(i*size_of::<Txo>()) as usize..(i*size_of::<Txo>()+8) as usize]),
                address: [0; 32]
            };
            txo.address.clone_from_slice(&output_bytes[(i*size_of::<Txo>()+8) as usize..(i*size_of::<Txo>()+40) as usize]);
            outputs.push(txo);
        }

        let mut tx = Tx {
            inputs: inputs,
            outputs: outputs,
            timestamp: timestamp,
            hash: [0; 32]
        };
        tx.hash.clone_from_slice(&hash);

        tx
    }

    pub fn to_vec(&self) -> Vec<u8>
    {
        let mut array = vec![];
        array.extend_from_slice(&self.hash);
        let mut ts_buf = [0; NBYTES_U64];
        BigEndian::write_u64(&mut ts_buf, self.timestamp);
        array.extend_from_slice(&ts_buf);

        let mut inplen_buf = [0; NBYTES_U32];
        BigEndian::write_u32(&mut inplen_buf, self.inputs.len() as u32);
        array.extend_from_slice(&inplen_buf);

        for txi in self.inputs.iter()
        {
            array.extend_from_slice(&txi.src_hash);
            let mut idx_buf = [0; NBYTES_U64];
            BigEndian::write_u64(&mut idx_buf, txi.src_idx);
            array.extend_from_slice(&idx_buf);
            array.extend_from_slice(&txi.signature);
        }

        let mut outlen_buf = [0; NBYTES_U32];
        BigEndian::write_u32(&mut outlen_buf, self.outputs.len() as u32);
        array.extend_from_slice(&outlen_buf);

        for txo in self.outputs.iter()
        {
            let mut amt_buf = [0; NBYTES_U64];
            BigEndian::write_u64(&mut amt_buf, txo.amount);
            array.extend_from_slice(&amt_buf);
            array.extend_from_slice(&txo.address);
        }

        array
    }

    fn gen_hash(&mut self) -> Vec<u8>
    {
        let mut txi_buf: Vec<u8> = vec![];
        for x in &self.inputs
        {
            let mut buf = [0; NBYTES_U64];
            BigEndian::write_u64(&mut buf, x.src_idx);
            txi_buf.extend_from_slice(&x.src_hash);
            txi_buf.extend_from_slice(&buf);
            txi_buf.extend_from_slice(&x.signature);
        }
        let mut txo_buf: Vec<u8> = vec![];
        for x in &self.outputs
        {
            let mut buf = [0; NBYTES_U64];
            BigEndian::write_u64(&mut buf, x.amount);
            txo_buf.extend_from_slice(&buf);
            txo_buf.extend_from_slice(&x.address);
        }
        let mut tms_buf = [0; NBYTES_U64];
        BigEndian::write_u64(&mut tms_buf, self.timestamp);
        let mut txn_buf = vec![];
        txn_buf.extend_from_slice(&mut tms_buf);
        txn_buf.extend_from_slice(&txi_buf);
        txn_buf.extend_from_slice(&txo_buf);
        digest::digest(&digest::SHA256, &txn_buf).as_ref().to_vec()
    }
}
