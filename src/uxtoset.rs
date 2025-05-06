use crate::{blockchain::Blockchain, error::Result};
use std::fs::remove_dir_all;
use sled;

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
}