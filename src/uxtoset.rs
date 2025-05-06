use crate::{blockchain::Blockchain, error::Result, tx::TXOutput};
use sled;
use std::fs::remove_dir_all;

pub struct UTXOSet {
    pub blockchain: Blockchain,
}

impl UTXOSet {
    pub fn reindex(&self) -> Result<()> {
        remove_dir_all("data/utxos")?;
        let db = sled::open("data/utxos")?;

        let utxos = self.blockchain.find_UTXO()?;

        for (txid, outs) in utxos {
            db.insert(txid.as_bytes(), bincode::serialize(&outs)?)?;
        }

        db.flush()?;
        Ok(())
    }

    pub fn update(&self, block: &Block) -> Result<()> {
        let db = sled::open("data/utxos")?;

        for tx in block.get_transactions() {
            if !tx.is_coinbase() {
                for vin in &tx.vin {
                    let mut update_outputs = TXOutputs {
                        outputs: Vec::new(),
                    };
                    let outs: TXOutputs = bincode::deserialize(&db.get(&vin.txid)?.unwrap())?;
                    for out_idx in 0..outs.outputs.len() {
                        if out_idx != vin.vout as usize {
                            update_outputs.outputs.push(outs.outputs[out_idx].clone());
                        }
                    }

                    if update_outputs.outputs.is_empty() {
                        db.remove(&vin.txid)?;
                    } else {
                        db.insert(vin.txid.as_bytes(), bincode::serialize(&update_outputs)?)?;
                    }
                }
            }
            let mut new_outputs = TXOutputs {
                outputs: Vec::new(),
            };

            for out in &tx.vout {
                new_outputs.outputs.push(out.clone());
            }

            db.insert(tx.id.as_bytes(), bincode::serialize(&new_outputs)?)?;
        }

        db.flush()?;
        Ok(())
    }

    pub fn count_transactions(&self) -> Result<i32> {
        let mut counter = 0;
        let db = sled::Open("data/utxos")?;
        for kv in db.iter() {
            kv?;
            counter += 1;
        }
        Ok(counter)
    }
}
