use cli::Cli;
use error::Result;

mod block;
mod blockchain;
mod cli;
mod error;
mod server;
mod transaction;
mod utxoset;
mod wallet;

fn main() -> Result<()> {
    let mut cli = Cli::new()?;
    cli.run()?;

    Ok(())
}
