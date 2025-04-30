use crate::error::Result;
use crate::block::{Block, TARGET_LEN};

#[derive(Debug)]
pub struct Blockchain {
    blocks: Vec<Block>,
}
impl Blockchain {
    pub fn new() -> Self {
        Blockchain {
            blocks: vec![Block::new_genesis_block()],
        }
    }

    pub fn add_block(&mut self, data: String) -> Result<()> {
        let prev = self.blocks.last().unwrap();
        let new_block = Block::new(data, prev.get_hash(), TARGET_LEN)?;
        self.blocks.push(new_block);
        Ok(())
    }
}