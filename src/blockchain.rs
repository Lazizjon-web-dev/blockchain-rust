use super::*;
use crate::{block::*, transaction::*};
use bincode::{deserialize, serialize};
use failure::format_err;
use log::info;
use sled::{open, Db};
use std::{collections::HashMap, fs::remove_dir_all};

#[derive(Debug, Clone)]
pub struct Blockchain {
    tip: String,
    db: Db,
}

pub struct BlockchainIterator<'a> {
    tip: String,
    bc: &'a Blockchain,
}

impl Blockchain {
    pub fn new() -> Result<Self> {
        info!("Opening blockchain");

        let db: Db = open("data/blocks")?;
        let hash = db
            .get("LAST")?
            .expect("Must create a new block database first");
        info!("Found block database");

        let last_hash = String::from_utf8(hash.to_vec())?;
        Ok(Blockchain { tip: last_hash, db })
    }

    pub fn create_blockchain(address: String) -> Result<Self> {
        info!("Creating blockchain");
        if remove_dir_all("data/blocks").is_err() {
            info!("not exists any blocks to delete")
        }
        let db: Db = open("data/blocks")?;
        info!("Creating new block database");
        let cbtx = Transaction::new_coinbase(address, String::from("GENESIS_COINBASE"))?;
        let genesis: Block = Block::new_genesis_block(cbtx);
        db.insert(genesis.get_hash(), serialize(&genesis)?)?;
        db.insert("LAST", genesis.get_hash().as_bytes())?;
        let bc = Blockchain {
            tip: genesis.get_hash(),
            db: db.clone(),
        };
        bc.db.flush()?;

        Ok(bc)
    }

    pub fn mine_block(&mut self, transactions: Vec<Transaction>) -> Result<Block> {
        info!("Mining new block");

        for tx in &transactions {
            if !self.verify_transaction(tx)? {
                return Err(format_err!("ERROR: Invalid transaction"));
            }
        }

        let last_hash = self.db.get("LAST")?.unwrap();

        let new_block = Block::new(
            transactions,
            String::from_utf8(last_hash.to_vec())?,
            self.get_best_height()? + 1,
        )?;
        self.db
            .insert(new_block.get_hash(), serialize(&new_block)?)?;
        self.db.insert("LAST", new_block.get_hash().as_bytes())?;
        self.db.flush()?;

        self.tip = new_block.get_hash();
        Ok(new_block)
    }

    pub fn get_block(&self, hash: &str) -> Result<Block> {
        let data = self.db.get(hash.as_bytes())?.unwrap();
        let block = deserialize(&data)?;
        Ok(block)
    }

    pub fn add_block(&mut self, block: Block) -> Result<()> {
        let data = serialize(&block)?;
        if (self.db.get(block.get_hash())?).is_some() {
            return Ok(());
        }
        self.db.insert(block.get_hash(), data)?;

        let last_height = self.get_best_height()?;
        if block.get_height() > last_height {
            self.db.insert("LAST", block.get_hash().as_bytes())?;
            self.tip = block.get_hash();
            self.db.flush()?;
        }
        Ok(())
    }

    pub fn get_best_height(&self) -> Result<i32> {
        let last_hash = if let Some(h) = self.db.get("LAST")? {
            h
        } else {
            return Ok(-1);
        };
        let last_data = self.db.get(last_hash)?.unwrap();
        let last_block: Block = deserialize(&last_data)?;
        Ok(last_block.get_height())
    }

    pub fn get_block_hashes(&self) -> Vec<String> {
        let mut list = Vec::new();
        for b in self.iter() {
            list.push(b.get_hash());
        }
        list
    }

    pub fn iter(&self) -> BlockchainIterator {
        BlockchainIterator {
            tip: self.tip.clone(),
            bc: self,
        }
    }

    fn find_unspent_transactions(&self, address: &[u8]) -> Vec<Transaction> {
        let mut spent_TXOs: HashMap<String, Vec<i32>> = HashMap::new();
        let mut unspend_TXOs: Vec<Transaction> = Vec::new();

        for block in self.iter() {
            for tx in block.get_transactions() {
                for index in 0..tx.vout.len() {
                    if let Some(ids) = spent_TXOs.get(&tx.id)
                        && ids.contains(&(index as i32))
                    {
                        continue;
                    }

                    if tx.vout[index].is_locked_with_key(address) {
                        unspend_TXOs.push(tx.to_owned());
                    }
                }

                if !tx.is_coinbase() {
                    for i in &tx.vin {
                        if i.can_unlock_output_with(address) {
                            match spent_TXOs.get_mut(&i.txid) {
                                Some(v) => {
                                    v.push(i.vout);
                                }
                                None => {
                                    spent_TXOs.insert(i.txid.clone(), vec![i.vout]);
                                }
                            }
                        }
                    }
                }
            }
        }

        unspend_TXOs
    }

    pub fn find_UTXO(&self) -> HashMap<String, TXOutputs> {
        let mut utxos: HashMap<String, TXOutputs> = HashMap::new();
        let mut spend_txos: HashMap<String, Vec<i32>> = HashMap::new();
        for block in self.iter() {
            for tx in block.get_transactions() {
                for index in 0..tx.vout.len() {
                    if let Some(ids) = spend_txos.get(&tx.id)
                        && ids.contains(&(index as i32))
                    {
                        continue;
                    }

                    match utxos.get_mut(&tx.id) {
                        Some(v) => {
                            v.outputs.push(tx.vout[index].clone());
                        }
                        None => {
                            utxos.insert(
                                tx.id.clone(),
                                TXOutputs {
                                    outputs: vec![tx.vout[index].clone()],
                                },
                            );
                        }
                    }
                }

                if !tx.is_coinbase() {
                    for i in &tx.vin {
                        match spend_txos.get_mut(&i.txid) {
                            Some(v) => {
                                v.push(i.vout);
                            }
                            None => {
                                spend_txos.insert(i.txid.clone(), vec![i.vout]);
                            }
                        }
                    }
                }
            }
        }
        utxos
    }

    pub fn find_transaction(&self, id: &str) -> Result<Transaction> {
        for block in self.iter() {
            for tx in block.get_transactions() {
                if tx.id == id {
                    return Ok(tx.clone());
                }
            }
        }

        Err(format_err!("Transaction is not found"))
    }

    pub fn sign_transaction(&self, tx: &mut Transaction, private_key: &[u8]) -> Result<()> {
        let prev_TXs = self.get_prev_tx_map(tx)?;
        tx.sign(private_key, prev_TXs)?;
        Ok(())
    }

    pub fn verify_transaction(&self, tx: &Transaction) -> Result<bool> {
        let prev_TXs = self.get_prev_tx_map(tx)?;
        tx.verify(prev_TXs)
    }

    fn get_prev_tx_map(&self, tx: &Transaction) -> Result<HashMap<String, Transaction>> {
        let mut prev_TXs = HashMap::new();
        for vin in &tx.vin {
            let prev_TX = self.find_transaction(&vin.txid)?;
            prev_TXs.insert(prev_TX.id.clone(), prev_TX);
        }
        Ok(prev_TXs)
    }
}

impl<'a> Iterator for BlockchainIterator<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(encode_block) = self.bc.db.get(&self.tip) {
            return match encode_block {
                Some(encode_block) => {
                    if let Ok(block) = deserialize::<Block>(&encode_block) {
                        self.tip = block.get_prev_hash();
                        Some(block)
                    } else {
                        None
                    }
                }
                None => None,
            };
        }
        None
    }
}
