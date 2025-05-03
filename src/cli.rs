use crate::blockchain::Blockchain;
use crate::error::Result;
use clap::{Command, arg};

pub struct Cli {
    bc: Blockchain,
}

impl Cli {
    pub fn new() -> Result<Self> {
        Ok(Cli {
            bc: Blockchain::new()?,
        })
    }
    pub fn run(&mut self) -> Result<()> {
        let matches = Command::new("Blockchain CLI")
            .version("1.0")
            .author("Lazizjon-web-dev")
            .about("A simple CLI for interacting with a blockchain")
            .subcommand(Command::new("print").about("Print the blockchain"))
            .subcommand(
                Command::new("add")
                    .about("Add a new block")
                    .arg(arg!(<DATA>" 'the blockchain data'")),
            )
            .get_matches();
        if let Some(ref matches) = matches.subcommand_matches("add") {
            if let Some(c) = matches.get_one::<String>("DATA") {
                self.add_block(c.clone())?;
                println!("Added block with data: {}", c);
            } else {
                println!("No data provided for the block.");
            }
        }

        if let Some(_) = matches.subcommand_matches("print") {
            self.print_chain()?;
        }

        Ok(())
    }

    fn add_block(&mut self, data: String) -> Result<()> {
        self.bc.add_block(data)?;
        Ok(())
    }
    fn print_chain(&self) -> Result<()> {
        for block in self.bc.iter() {
            println!("Block: {:?}", block);
        }
        Ok(())
    }
}
