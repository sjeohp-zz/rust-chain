use transaction::*;
use block::*;
use peer::*;

extern crate postgres;
use self::postgres::{Connection, TlsMode};

#[cfg(test)]
const DB_URL: &'static str = "postgresql://chaintest@localhost:5432/chaindbtest";

#[cfg(not(test))]
const DB_URL: &'static str = "postgresql://chain@localhost:5432/chaindb";

pub enum DatabaseInsertionError
{
    ValueExists,
    // Unknown
}

pub fn conn() -> Connection
{
    Connection::connect(DB_URL, TlsMode::None).expect("Unable to connect to database")
}

pub fn peers(db: &Connection) -> Vec<Peer>
{
    db.query(
        "SELECT ip, port, timestamp FROM peers ORDER BY timestamp DESC;",
        &[])
        .unwrap()
        .iter()
        .map(|row| Peer::new(
            row.get(0),
            row.get(1),
            row.get(2),
            None))
        .collect()
}

pub fn upsert_peer(peer: &Peer, db: &Connection) -> Result<(), DatabaseInsertionError>
{
    let mut result = Ok(());
    db.execute("BEGIN WORK;", &[]).unwrap();
    db.execute("LOCK TABLE blocks IN SHARE ROW EXCLUSIVE MODE;", &[]).unwrap();
    if db.execute(
        "SELECT 1 FROM peers WHERE ip = $1 AND port = $2",
        &[&peer.ip, &peer.port])
        .unwrap() != 1
    {
        db.execute(
            "INSERT INTO peers (ip, port, timestamp) SELECT $1, $2, $3",
            &[&peer.ip, &peer.port, &peer.timestamp])
            .unwrap();
    }
    else
    {
        db.execute(
            "UPDATE peers SET timestamp = $1 WHERE ip = $2 AND port = $3",
            &[&peer.timestamp, &peer.ip, &peer.port])
            .unwrap();
        result = Err(DatabaseInsertionError::ValueExists);
    }
    result
}

pub fn blockchain(db: &Connection) -> Vec<Block>
{
    db.query(
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
        .collect()
}

pub fn pending_txs(db: &Connection) -> Vec<Transaction>
{
    db.query(
        "SELECT hash, public_key, timestamp, block FROM transactions WHERE block NOT IN (SELECT block_hash FROM blocks);",
        &[])
        .unwrap()
        .iter()
        .map(|row|
            Transaction::new_with_hash(
                &(row.get::<usize, Vec<u8>>(0)),
                &(row.get::<usize, Vec<u8>>(1)),
                row.get(2),
            ))
        .collect()
}

pub fn tx_inputs(tx: &Transaction, db: &Connection) -> Vec<TxInput>
{
    db.query(
        "SELECT src_hash, src_idx, signature FROM tx_inputs WHERE tx = $1;",
        &[&tx.hash.as_ref()])
        .unwrap()
        .iter()
        .map(|row|
            TxInput::from_stored(
                &(row.get::<usize, Vec<u8>>(0)),
                row.get(1),
                &(row.get::<usize, Vec<u8>>(2))
            ))
        .collect()
}

pub fn tx_outputs(tx: &Transaction, db: &Connection) -> Vec<TxOutput>
{
    db.query(
        "SELECT amount, address FROM tx_outputs WHERE tx = $1;",
        &[&tx.hash.as_ref()])
        .unwrap()
        .iter()
        .map(|row|
            TxOutput::new(
                row.get(0),
                &(row.get::<usize, Vec<u8>>(1))
            ))
        .collect()
}

pub fn block(hash: &[u8], db: &Connection) -> Option<Block>
{
    let blocks: Vec<Block> = db.query(
        "SELECT txs_hash, parent_hash, target, timestamp, nonce, block_hash FROM blocks WHERE block_hash = $1;",
        &[&hash.as_ref()])
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
    blocks.first().map_or(None, |x| Some(x.clone()))
}

pub fn insert_block(block: &Block, db: &Connection) -> Result<(), DatabaseInsertionError>
{
    let mut result = Ok(());
    db.execute("BEGIN WORK;", &[]).unwrap();
    db.execute("LOCK TABLE blocks IN SHARE ROW EXCLUSIVE MODE;", &[]).unwrap();
    if db.execute(
        "SELECT 1 FROM blocks WHERE block_hash = $1",
        &[&block.block_hash.as_ref()])
        .unwrap() != 1
    {
        db.execute(
            "INSERT INTO blocks (txs_hash, parent_hash, target, timestamp, nonce, block_hash) SELECT $1, $2, $3, $4, $5, $6",
            &[&block.txs_hash.as_ref(), &block.parent_hash.as_ref(), &block.target.as_ref(), &block.timestamp, &block.nonce, &block.block_hash.as_ref()])
            .unwrap();

        // db.execute("LOCK TABLE transactions IN SHARE ROW EXCLUSIVE MODE;", &[]).unwrap();
        for tx in block.txs.iter()
        {
            if db.execute(
                "SELECT 1 FROM transactions WHERE hash = $1",
                &[&tx.hash.as_ref()])
                .unwrap() != 1
            {
                // TRANSACTION DOESN'T EXIST LOCALLY, MUST HAVE RECEIVED THIS BLOCK FROM A PEER
                db.execute(
                    "INSERT INTO transactions (hash, timestamp, block) SELECT $1, $2, $3",
                    &[&tx.hash.as_ref(), &tx.timestamp, &block.block_hash.as_ref()])
                    .unwrap();

                for txi in tx.inputs.iter()
                {
                    db.execute(
                        "INSERT INTO tx_inputs (src_hash, src_idx, signature, tx) SELECT $1, $2, $3, $4",
                        &[&txi.src_hash.as_ref(), &txi.src_idx, &txi.signature.as_ref(), &tx.hash.as_ref()])
                        .unwrap();
                }
                for (i, txo) in tx.outputs.iter().enumerate()
                {
                    db.execute(
                        "INSERT INTO tx_outputs (idx, amount, address, tx) SELECT $1, $2, $3, $4",
                        &[&(i as i64), &txo.amount, &txo.address.as_ref(), &tx.hash.as_ref()])
                        .unwrap();
                }
            }
            else
            {
                // TRANSACTION IS ALREADY STORED/PENDING; ADD IT TO THE BLOCK
                db.execute(
                    "UPDATE transactions SET block = $1 WHERE hash = $2",
                    &[&block.block_hash.as_ref(), &tx.hash.as_ref()])
                    .unwrap();
            }
        }
    }
    else
    {
        result = Err(DatabaseInsertionError::ValueExists);
    }
    db.execute("COMMIT WORK;", &[]).unwrap();
    result
}

pub fn insert_transaction(tx: &Transaction, db: &Connection) -> Result<(), DatabaseInsertionError>
{
    let mut result = Ok(());
    db.execute("BEGIN WORK;", &[]).unwrap();
    db.execute("LOCK TABLE transactions IN SHARE ROW EXCLUSIVE MODE;", &[]).unwrap();
    if db.execute(
        "SELECT 1 FROM transactions WHERE hash = $1",
        &[&tx.hash.as_ref()])
        .unwrap() != 1
    {
        db.execute(
            "INSERT INTO transactions (hash, public_key, timestamp) SELECT $1, $2, $3",
            &[&tx.hash.as_ref(), &tx.public_key.as_ref(), &tx.timestamp])
            .unwrap();
        for txi in tx.inputs.iter()
        {
            db.execute(
                "INSERT INTO tx_inputs (src_hash, src_idx, signature, tx) SELECT $1, $2, $3, $4",
                &[&txi.src_hash.as_ref(), &txi.src_idx, &txi.signature.as_ref(), &tx.hash.as_ref()])
                .unwrap();
        }
        for (i, txo) in tx.outputs.iter().enumerate()
        {
            db.execute(
                "INSERT INTO tx_outputs (idx, amount, address, tx) SELECT $1, $2, $3, $4",
                &[&(i as i64), &txo.amount, &txo.address.as_ref(), &tx.hash.as_ref()])
                .unwrap();
        }
    }
    else
    {
        result = Err(DatabaseInsertionError::ValueExists);
    }
    db.execute("COMMIT WORK;", &[]).unwrap();
    result
}

pub fn unspent_outputs(public_key: &[u8], db: &Connection) -> Vec<TxOutput>
{
    let result = db.query(
        "SELECT
        tx_outputs.amount, tx_outputs.address
        FROM
        tx_outputs, transactions, blocks
        WHERE
        tx_outputs.address = $1 AND
        tx_outputs.tx = transactions.hash AND
        transactions.block = blocks.block_hash AND
        tx_outputs.id NOT IN
        (
            SELECT
            tx_outputs.id
            FROM
            tx_outputs, tx_inputs
            WHERE
            tx_inputs.src_hash = tx_outputs.tx AND
            tx_inputs.src_idx = tx_outputs.idx
        )",
        &[&public_key])
        .unwrap()
        .iter()
        .map(|row|
            TxOutput::new(
                row.get(0),
                &(row.get::<usize, Vec<u8>>(1))
            ))
        .collect();
    result
}
