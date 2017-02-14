use transaction::*;
use database;

pub fn balance(public_key: &[u8]) -> i64
{
    database::unspent_outputs(public_key, &database::conn()).iter().fold(0, |sum, txo| sum + txo.amount)
}
