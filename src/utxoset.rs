use super::*;
use crate::{block::Block, blockchain::Blockchain, transaction::*};
use bincode::{deserialize, serialize};
use log::info;
use sled::open;
use std::{collections::HashMap, fs::remove_dir_all};

pub struct UTXOSet {
    pub blockchain: Blockchain,
}

impl UTXOSet {
    pub fn reindex(&self) -> Result<()> {
        if remove_dir_all("data/utxos").is_err() {
            info!("not exists any utxos to delete")
        }
        let db = open("data/utxos")?;

        let utxos = self.blockchain.find_UTXO();

        for (txid, outs) in utxos {
            db.insert(txid.as_bytes(), serialize(&outs)?)?;
        }

        db.flush()?;
        Ok(())
    }

    pub fn update(&self, block: &Block) -> Result<()> {
        let db = open("data/utxos")?;

        for tx in block.get_transactions() {
            if !tx.is_coinbase() {
                for vin in &tx.vin {
                    let mut update_outputs = TXOutputs {
                        outputs: Vec::new(),
                    };
                    let outs: TXOutputs = deserialize(&db.get(&vin.txid)?.unwrap())?;
                    for out_idx in 0..outs.outputs.len() {
                        if out_idx != vin.vout as usize {
                            update_outputs.outputs.push(outs.outputs[out_idx].clone());
                        }
                    }

                    if update_outputs.outputs.is_empty() {
                        db.remove(&vin.txid)?;
                    } else {
                        db.insert(vin.txid.as_bytes(), serialize(&update_outputs)?)?;
                    }
                }
            }
            let mut new_outputs = TXOutputs {
                outputs: Vec::new(),
            };

            for out in &tx.vout {
                new_outputs.outputs.push(out.clone());
            }

            db.insert(tx.id.as_bytes(), serialize(&new_outputs)?)?;
        }

        db.flush()?;
        Ok(())
    }

    pub fn count_transactions(&self) -> Result<i32> {
        let mut counter = 0;
        let db = open("data/utxos")?;
        for kv in db.iter() {
            kv?;
            counter += 1;
        }
        Ok(counter)
    }

    pub fn find_spendable_outputs(
        &self,
        pub_hash_key: &[u8],
        amount: i32,
    ) -> Result<(i32, HashMap<String, Vec<i32>>)> {
        let mut unspent_outputs: HashMap<String, Vec<i32>> = HashMap::new();
        let mut accumulated: i32 = 0;
        let db = open("data/utxos")?;
        for kv in db.iter() {
            let (key, value) = kv?;
            let txid = String::from_utf8(key.to_vec())?;
            let outs: TXOutputs = deserialize(&value)?;

            for out_idx in 0..outs.outputs.len() {
                if outs.outputs[out_idx].is_locked_with_key(pub_hash_key) && accumulated < amount {
                    accumulated += outs.outputs[out_idx].value;
                    match unspent_outputs.get_mut(&txid) {
                        Some(v) => v.push(out_idx as i32),
                        None => {
                            unspent_outputs.insert(txid.clone(), vec![out_idx as i32]);
                        }
                    }
                }
            }
        }
        Ok((accumulated, unspent_outputs))
    }

    pub fn find_UTXO(&self, pub_hash_key: &[u8]) -> Result<TXOutputs> {
        let mut utxos = TXOutputs {
            outputs: Vec::new(),
        };
        let db = open("data/utxos")?;
        for kv in db.iter() {
            let (_, value) = kv?;
            let outs: TXOutputs = deserialize(&value)?;

            for out in outs.outputs {
                if out.is_locked_with_key(pub_hash_key) {
                    utxos.outputs.push(out.clone());
                }
            }
        }
        Ok(utxos)
    }
}
