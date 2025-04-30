use crypto::digest::Digest;
use crypto::sha2::Sha256;
use log::info;
use std::time::SystemTime;

pub type Result<T> = std::result::Result<T, failure::Error>;

const TARGET_LEN: usize = 4;

#[derive(Debug, Clone)]
pub struct Block {
    timestamp: u128,
    transactions: String,
    prev_block_hash: String,
    hash: String,
    height: usize,
    nonce: i32,
}

#[derive(Debug)]
pub struct Blockchain {
    blocks: Vec<Block>,
}

impl Block {
    pub fn new(data: String, prev_block_hash: String, height: usize) -> Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis();
        let mut block = Block {
            timestamp,
            transactions: data,
            prev_block_hash,
            hash: String::new(),
            height,
            nonce: 0,
        };
        block.run_proof_of_work()?;
        Ok(block)
    }

    pub fn new_genesis_block() -> Self {
        Block {
            timestamp: 0,
            transactions: String::from("Genesis Block"),
            prev_block_hash: String::from("0"),
            hash: String::from("0"),
            height: 0,
            nonce: 0,
        }
    }

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    fn run_proof_of_work(&mut self) -> Result<()> {
        info!("Mining the block");
        while !self.validate()? {
            self.nonce += 1;
        }
        let data = self.prepare_hash_data().unwrap();
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        self.hash = hasher.result_str();
        Ok(())
    }

    fn prepare_hash_data(&self) -> Result<Vec<u8>> {
        let content = (
            self.prev_block_hash.clone(),
            self.transactions.clone(),
            self.timestamp,
            TARGET_LEN,
            self.nonce,
        );
        let mut bytes: Vec<u8> = content
            .0
            .as_bytes()
            .iter()
            .chain(content.1.as_bytes().iter())
            .chain(content.2.to_ne_bytes().iter())
            .chain(content.3.to_ne_bytes().iter())
            .chain(content.4.to_ne_bytes().iter())
            .copied()
            .collect();
        Ok(bytes)
    }

    fn validate(&self) -> Result<bool> {
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        let mut vec1: Vec<u8> = vec![];
        vec1.resize(TARGET_LEN, 0 as u8);
        println!("vec1: {:?}", vec1);
        Ok(&hasher.result_str()[0..TARGET_LEN] == String::from_utf8(vec1)?)
    }
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
