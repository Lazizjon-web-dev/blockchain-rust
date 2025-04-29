pub struct Block {
    timestamp: u128,
    transactions: String,
    prev_block_hash: String,
    hash: String,
    height: usize,
    nonce: i32,
}

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
}