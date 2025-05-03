use crate::block::{Block, TARGET_LEN};
use crate::error::Result;
use crate::transaction::Transaction;
use log::info;
use sled::{self, Db};

#[derive(Debug, Clone)]
pub struct Blockchain {
    current_hash: String,
    db: sled::Db,
}

pub struct BlockchainIterator<'a> {
    current_hash: String,
    bc: &'a Blockchain,
}

impl Blockchain {
    pub fn new() -> Result<Self> {
        info!("Opening blockchain");

        let db: Db = sled::open("data/blocks")?;
        let hash = db
            .get("LAST")?
            .expect("Must create a new block database first");
        info!("Found block database");

        let last_hash = String::from_utf8(hash.to_vec())?;
        Ok(Blockchain {
            current_hash: last_hash,
            db,
        })
    }

    pub fn create_blockchain(address: String) -> Result<Self> {
        info!("Creating blockchain");
        let db: Db = sled::open("data/blocks")?;
        info!("Creating new block database");
        let cbtx = Transaction::new_coinbase(address, String::from("GENESIS_COINBASE"))?;
        let genesis: Block = Block::new_genesis_block(cbtx);
        db.insert(genesis.get_hash(), bincode::serialize(&genesis)?)?;
        db.insert("LAST", genesis.get_hash().as_bytes())?;
        let bc = Blockchain {
            current_hash: genesis.get_hash(),
            db: db.clone(),
        };
        bc.db.flush()?;

        Ok(bc)
    }

    pub fn add_block(&mut self, data: String) -> Result<()> {
        let last_hash = self.db.get("LAST")?.unwrap();
        let last_hash = String::from_utf8(last_hash.to_vec())?;
        let new_block = Block::new(data, last_hash, TARGET_LEN)?;
        self.db.insert(new_block.get_hash(), bincode::serialize(&new_block)?)?;
        self.db.insert("LAST", new_block.get_hash().as_bytes())?;
        self.current_hash = new_block.get_hash();
        Ok(())
    }

    pub fn iter(&self) -> BlockchainIterator {
        BlockchainIterator {
            current_hash: self.current_hash.clone(),
            bc: &self,
        }
    }
}

impl<'a> Iterator for BlockchainIterator<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(encode_block) = self.bc.db.get(&self.current_hash) {
            return match encode_block {
                Some(encode_block) => {
                    if let Ok(block) = bincode::deserialize::<Block>(&encode_block) {
                        self.current_hash = block.get_prev_hash();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_block() {
        let mut bc = Blockchain::new().unwrap();
        bc.add_block("data 1".to_string()).unwrap();
        bc.add_block("data 2".to_string()).unwrap();
        bc.add_block("data 3".to_string()).unwrap();

        for item in bc.iter() {
            println!("item {:?}", item);
        }
    }
}
