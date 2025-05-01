use crate::blockchain::Blockchain;
use crate::error::Result;
use rustyline::DefaultEditor;

pub struct Cli {
    bc: Blockchain,
    rl: DefaultEditor,
}

impl Cli {
    pub fn new() -> Result<Self> {
        Ok(Cli { bc: Blockchain::new()?, rl: DefaultEditor::new()? })
    }
    pub fn run(&mut self) -> Result<()> {
        Ok(())
    }
}