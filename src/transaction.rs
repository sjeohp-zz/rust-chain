extern crate byteorder;
use self::byteorder::{ByteOrder, LittleEndian};

use util::{NBYTES_U64, NBYTES_U32};

use std::mem::size_of;

use wallet;
use crypto;

pub struct TxInput
{
    pub src_hash:   [u8; 32],
    pub src_idx:    i64,
    pub signature:  [u8; 64],
}

impl Clone for TxInput
{
    #[inline]
    fn clone(&self) -> TxInput
    {
        let mut txi = TxInput {
            src_hash: [0; 32],
            src_idx: self.src_idx,
            signature: [0; 64]
        };
        txi.src_hash.clone_from_slice(&self.src_hash);
        txi.signature.clone_from_slice(&self.signature);
        txi
    }
}

impl PartialEq for TxInput
{
    fn eq(&self, other: &TxInput) -> bool
    {
        let mut sigeq = true;
        for i in 0..64 { sigeq = self.signature[i] == other.signature[i]; }

        self.src_hash == other.src_hash && self.src_idx == other.src_idx && sigeq
    }
}

impl TxInput
{
    pub fn new(
        src_hash: &[u8],
        src_idx: i64) -> TxInput
    {
        let mut clo = TxInput {
            src_hash: [0; 32],
            src_idx: src_idx,
            signature: [0; 64]
        };
        clo.src_hash.clone_from_slice(&src_hash);
        clo
    }

    pub fn from_stored(
        src_hash: &[u8],
        src_idx: i64,
        signature: &[u8]) -> TxInput
    {
        let mut clo = TxInput {
            src_hash: [0; 32],
            src_idx: src_idx,
            signature: [0; 64]
        };
        clo.src_hash.clone_from_slice(&src_hash);
        clo.signature.clone_from_slice(&signature);
        clo
    }
}

#[derive(PartialEq)]
pub struct TxOutput
{
    pub amount:     i64,
    pub address:    [u8; 32],
}

impl Clone for TxOutput
{
    #[inline]
    fn clone(&self) -> TxOutput
    {
        let mut clo = TxOutput {
            amount: self.amount,
            address: [0; 32]
        };
        clo.address.clone_from_slice(&self.address);
        clo
    }
}

impl TxOutput
{
    pub fn new(
        amount: i64,
        address: &[u8]) -> TxOutput
    {
        let mut txo = TxOutput {
            amount: amount,
            address: [0; 32]
        };
        txo.address.clone_from_slice(&address);
        txo
    }
}

#[derive(PartialEq)]
pub struct Transaction
{
    pub hash:       [u8; 32],
    pub timestamp:  i64,
    pub inputs:     Vec<TxInput>,
    pub outputs:    Vec<TxOutput>,
}

impl Clone for Transaction
{
    #[inline]
    fn clone(&self) -> Transaction
    {
        let mut clo = Transaction {
            hash: [0; 32],
            timestamp: self.timestamp,
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
        };
        clo.hash.clone_from_slice(&self.hash);
        clo
    }
}

impl Transaction
{
    pub fn new_with_hash(
        hash: &[u8],
        timestamp: i64) -> Transaction
    {
        let mut tx = Transaction {
            hash: [0; 32],
            timestamp: timestamp,
            inputs: vec![],
            outputs: vec![],
        };
        tx.hash.clone_from_slice(hash);
        tx
    }

    pub fn new(
        inputs: Vec<TxInput>,
        outputs: Vec<TxOutput>,
        timestamp: i64) -> Transaction
    {
        let mut tx = Transaction {
            hash: [0; 32],
            timestamp: timestamp,
            inputs: inputs,
            outputs: outputs,
        };
        tx.sign_inputs();
        tx.hash_contents();
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
        let mut txn_buf = vec![];
        txn_buf.extend_from_slice(&txi_buf);
        txn_buf.extend_from_slice(&txo_buf);
        txn_buf
    }

    fn sign_inputs(&mut self)
    {
        let signature = wallet::get_signature(&self.signable_vec());
        for txi in &mut self.inputs
        {
            txi.signature.clone_from_slice(&signature);
        }
    }

    fn hashable_vec(&self) -> Vec<u8>
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

    fn hash_contents(&mut self)
    {
        let buf = &self.hashable_vec();
        self.hash.clone_from_slice(&crypto::digest_sha256(buf));
    }

    pub fn verify(&mut self) -> bool
    {
        let mut valid = true;
        for txi in &mut self.inputs.iter()
        {
            valid = wallet::verify_signature(&self.signable_vec(), &txi.signature);
        }
        return valid
    }

    pub fn from_slice(bytes: &[u8]) -> Transaction
    {
        let hash = &bytes[..32];
        let timestamp = LittleEndian::read_i64(&bytes[32..40]);
        let mut idx: usize = 40;
        let inp_len = (LittleEndian::read_u32(&bytes[idx..idx+4]) as usize) * size_of::<TxInput>();
        idx += 4;
        let input_bytes = &bytes[idx..idx+(inp_len)];
        idx += inp_len;
        let out_len = (LittleEndian::read_u32(&bytes[idx..idx+4]) as usize) * size_of::<TxOutput>();
        idx += 4;
        let output_bytes = &bytes[idx..idx+(out_len)];

        let mut inputs = vec![];
        for i in 0..(inp_len / size_of::<TxInput>())
        {
            let mut txi = TxInput {
                src_hash: [0; 32],
                src_idx: LittleEndian::read_i64(&input_bytes[(i*size_of::<TxInput>()+32)..(i*size_of::<TxInput>()+40)]),
                signature: [0; 64]
            };
            txi.src_hash.clone_from_slice(&input_bytes[(i*size_of::<TxInput>()) as usize..(i*size_of::<TxInput>()+32) as usize]);
            txi.signature.clone_from_slice(&input_bytes[(i*size_of::<TxInput>()+40) as usize..(i*size_of::<TxInput>()+104) as usize]);
            inputs.push(txi);
        }

        let mut outputs = vec![];
        for i in 0..(out_len / size_of::<TxOutput>())
        {
            let mut txo = TxOutput {
                amount: LittleEndian::read_i64(&output_bytes[(i*size_of::<TxOutput>()) as usize..(i*size_of::<TxOutput>()+8) as usize]),
                address: [0; 32]
            };
            txo.address.clone_from_slice(&output_bytes[(i*size_of::<TxOutput>()+8) as usize..(i*size_of::<TxOutput>()+40) as usize]);
            outputs.push(txo);
        }

        let mut tx = Transaction {
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
