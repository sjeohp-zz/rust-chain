extern crate ring;
extern crate untrusted;
extern crate byteorder;

use self::ring::{digest};

use self::byteorder::{ByteOrder, LittleEndian};

use util::{NBYTES_U64, NBYTES_U32};

use std::mem::size_of;

use wallet::*;

// #[derive(Clone, Debug, PartialEq)]
pub struct Txi
{
    pub src_hash:   [u8; 64],
    pub src_idx:    i64,
    pub signature:  [u8; 64],
}

// #[derive(Clone, Debug, PartialEq)]
pub struct Txo
{
    pub amount:     i64,
    pub address:    [u8; 32],
}

// #[derive(Clone, Debug, PartialEq)]
pub struct Tx
{
    pub hash:       [u8; 64],
    pub timestamp:  i64,
    pub inputs:     Vec<Txi>,
    pub outputs:    Vec<Txo>,
}

impl Tx
{
    pub fn new(
        inputs: Vec<Txi>,
        outputs: Vec<Txo>,
        timestamp: i64
    ) -> Tx
    {
        let mut tx = Tx {
            hash: [0; 64],
            timestamp: timestamp,
            inputs: inputs,
            outputs: outputs,
        };
        tx.sign();
        tx
    }

    fn signable_vec(&self) -> Vec<u8>
    {
        let mut txi_buf: Vec<u8> = vec![];
        for x in &self.inputs
        {
            let mut buf = [0; NBYTES_U64];
            LittleEndian::write_i64(&mut buf, x.src_idx);
            txi_buf.extend_from_slice(&x.src_hash);
            txi_buf.extend_from_slice(&buf);
        }
        let mut txo_buf: Vec<u8> = vec![];
        for x in &self.outputs
        {
            let mut buf = [0; NBYTES_U64];
            LittleEndian::write_i64(&mut buf, x.amount);
            txo_buf.extend_from_slice(&buf);
            txo_buf.extend_from_slice(&x.address);
        }
        let mut tms_buf = [0; NBYTES_U64];
        LittleEndian::write_i64(&mut tms_buf, self.timestamp);
        let mut txn_buf = vec![];
        txn_buf.extend_from_slice(&mut tms_buf);
        txn_buf.extend_from_slice(&txi_buf);
        txn_buf.extend_from_slice(&txo_buf);
        txn_buf
    }

    fn sign(&mut self)
    {
        let signature = signature(&self.signable_vec());

        self.hash.clone_from_slice(&signature);
        for txi in &mut self.inputs
        {
            txi.signature.clone_from_slice(&signature);
        }
    }

    pub fn verify(&mut self) -> bool
    {
        let mut public_key: [u8; 32] = [0; 32];
        let mut private_key: [u8; 32] = [0; 32];
        get_or_gen_wallet(&mut public_key, &mut private_key);

        let mut valid = verify(&self.signable_vec(), &self.hash, &public_key);
        for txi in &mut self.inputs
        {
            for i in 0..64
            {
                if self.hash[i] != txi.signature[i] { valid = false; }
            }
        }
        return valid
    }

    pub fn from_slice(bytes: &[u8]) -> Tx
    {
        let hash = &bytes[..64];
        let timestamp = LittleEndian::read_i64(&bytes[64..72]);
        let mut idx: usize = 72;
        let inp_len = (LittleEndian::read_u32(&bytes[idx..idx+4]) as usize) * size_of::<Txi>();
        idx += 4;
        let input_bytes = &bytes[idx..idx+(inp_len)];
        idx += inp_len;
        let out_len = (LittleEndian::read_u32(&bytes[idx..idx+4]) as usize) * size_of::<Txo>();
        idx += 4;
        let output_bytes = &bytes[idx..idx+(out_len)];

        let mut inputs = vec![];
        for i in 0..(inp_len / size_of::<Txi>())
        {
            let mut txi = Txi {
                src_hash: [0; 64],
                src_idx: LittleEndian::read_i64(&input_bytes[(i*size_of::<Txi>()+64)..(i*size_of::<Txi>()+72)]),
                signature: [0; 64]
            };
            txi.src_hash.clone_from_slice(&input_bytes[(i*size_of::<Txi>()) as usize..(i*size_of::<Txi>()+64) as usize]);
            txi.signature.clone_from_slice(&input_bytes[(i*size_of::<Txi>()+72) as usize..(i*size_of::<Txi>()+136) as usize]);
            inputs.push(txi);
        }

        let mut outputs = vec![];
        for i in 0..(out_len / size_of::<Txo>())
        {
            let mut txo = Txo {
                amount: LittleEndian::read_i64(&output_bytes[(i*size_of::<Txo>()) as usize..(i*size_of::<Txo>()+8) as usize]),
                address: [0; 32]
            };
            txo.address.clone_from_slice(&output_bytes[(i*size_of::<Txo>()+8) as usize..(i*size_of::<Txo>()+40) as usize]);
            outputs.push(txo);
        }

        let mut tx = Tx {
            inputs: inputs,
            outputs: outputs,
            timestamp: timestamp,
            hash: [0; 64]
        };
        tx.hash.clone_from_slice(&hash);

        tx
    }

    pub fn to_vec(&self) -> Vec<u8>
    {
        let mut array = vec![];
        array.extend_from_slice(&self.hash);
        let mut ts_buf = [0; NBYTES_U64];
        LittleEndian::write_i64(&mut ts_buf, self.timestamp);
        array.extend_from_slice(&ts_buf);

        let mut inplen_buf = [0; NBYTES_U32];
        LittleEndian::write_u32(&mut inplen_buf, self.inputs.len() as u32);
        array.extend_from_slice(&inplen_buf);

        for txi in self.inputs.iter()
        {
            array.extend_from_slice(&txi.src_hash);
            let mut idx_buf = [0; NBYTES_U64];
            LittleEndian::write_i64(&mut idx_buf, txi.src_idx);
            array.extend_from_slice(&idx_buf);
            array.extend_from_slice(&txi.signature);
        }

        let mut outlen_buf = [0; NBYTES_U32];
        LittleEndian::write_u32(&mut outlen_buf, self.outputs.len() as u32);
        array.extend_from_slice(&outlen_buf);

        for txo in self.outputs.iter()
        {
            let mut amt_buf = [0; NBYTES_U64];
            LittleEndian::write_i64(&mut amt_buf, txo.amount);
            array.extend_from_slice(&amt_buf);
            array.extend_from_slice(&txo.address);
        }

        array
    }
}
