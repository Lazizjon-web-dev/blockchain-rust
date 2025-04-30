use crate::blockchain::Blockchain;
use crate::error::Result;

pub struct Cli {
    bc: Blockchain,
}

impl Cli {
    pub fn new() -> Result<Self> {
        Ok(Cli { bc: Blockchain::new()? })
    }
}