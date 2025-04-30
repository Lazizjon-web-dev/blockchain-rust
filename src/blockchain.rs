use crate::error::Result;
use crate::block::{Block, TARGET_LEN};
use sled::{self, Db};

#[derive(Debug)]
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
        let db: Db = sled::open("data/blocks")?;
        match db.get("LAST")? {
            Some(hash) => {
                let last_hash = String::from_utf8(hash.to_vec())?;
                Ok(Blockchain {
                    current_hash: last_hash,
                    db,
                })
            }
            None=> {
                let block = Block::new_genesis_block();
                db.insert(block.get_hash(), block.serialize())?;
                db.insert("LAST", block.get_hash().as_bytes())?;
                let bc = Blockchain {
                    current_hash: block.get_hash(),
                    db,
                };
                bc.db.flush()?;
                Ok(bc)
            }
        }
    }

    pub fn add_block(&mut self, data: String) -> Result<()> {
        let last_hash = self.db.get("LAST")?.unwrap();
        let last_hash = String::from_utf8(last_hash.to_vec())?;
        let new_block = Block::new(data, last_hash, TARGET_LEN)?;
        self.db.insert(new_block.get_hash(), new_block.serialize())?;
        self.db.insert("LAST", new_block.get_hash().as_bytes())?;
        self.current_hash = new_block.get_hash();
        Ok(())
    }
}

impl<'a> Iterator for BlockchainIterator<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(encode_block) = self.bc.db.get(&self.current_hash) {
            return match encode_block {
                Some(encode_block) => {
                    if let Ok(block) = Block::deserialize(&encode_block) {
                        self.current_hash = block.get_prev_hash();
                        Some(block)
                    } else {
                        None
                    }
                }
                None => None
            };
        }
        None
    }
}