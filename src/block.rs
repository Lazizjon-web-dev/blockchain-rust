use crate::{error::Result, transaction::Transaction};
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use log::info;
use merkle_cbt::merkle_tree::{Merge, CBMT};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

pub const TARGET_HEXS: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    timestamp: u128,
    transactions: Vec<Transaction>,
    prev_block_hash: String,
    hash: String,
    nonce: i32,
    height: i32,
}

impl Block {
    pub fn new(
        transactions: Vec<Transaction>,
        prev_block_hash: String,
        height: i32,
    ) -> Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis();
        let mut block = Block {
            timestamp,
            transactions,
            prev_block_hash,
            hash: String::new(),
            nonce: 0,
            height,
        };
        block.run_proof_of_work()?;
        Ok(block)
    }

    pub fn new_genesis_block(coinbase: Transaction) -> Self {
        Block::new(vec![coinbase], String::new(), 0).expect("Failed to create genesis block")
    }

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    pub fn get_prev_hash(&self) -> String {
        self.prev_block_hash.clone()
    }

    pub fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }

    pub fn get_height(&self) -> i32 {
        self.height
    }

    fn run_proof_of_work(&mut self) -> Result<()> {
        info!("Mining the block");
        while !self.validate()? {
            self.nonce += 1;
        }
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        self.hash = hasher.result_str();
        Ok(())
    }

    fn hash_transactions(&mut self) -> Result<Vec<u8>> {
        let mut transactions = Vec::new();

        for tx in &mut self.transactions {
            transactions.push(tx.hash()?.as_bytes().to_owned());
        }

        let tree = CBMT::<Vec<u8>, MergeVu8>::build_merkle_tree(&transactions);

        Ok(tree.root())
    }

    fn prepare_hash_data(&mut self) -> Result<Vec<u8>> {
        let content = (
            self.prev_block_hash.clone(),
            self.hash_transactions()?,
            self.timestamp,
            TARGET_HEXS,
            self.nonce,
        );
        let bytes: Vec<u8> = bincode::serialize(&content)?;
        Ok(bytes)
    }

    fn validate(&mut self) -> Result<bool> {
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        let mut vec1: Vec<u8> = Vec::new();
        vec1.resize(TARGET_HEXS, 0 as u8);
        println!("vec1: {:?}", vec1);
        Ok(&hasher.result_str()[0..TARGET_HEXS] == String::from_utf8(vec1)?)
    }
}

struct MergeVu8 {}

impl Merge for MergeVu8 {
    type Item = Vec<u8>;

    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut hasher = Sha256::new();
        let mut data: Vec<u8> = left.clone();
        data.append(&mut right.clone());
        hasher.input(&data[..]);
        let mut result: [u8; 32] = [0; 32];
        hasher.result(&mut result);
        result.to_vec()
    }
}
