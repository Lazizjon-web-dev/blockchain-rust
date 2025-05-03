use cli::Cli;
use error::Result;

mod error;
mod blockchain;
mod block;
mod cli;

fn main() -> Result<()> {
    let mut cli = Cli::new()?;
    cli.run()?;

    Ok(())
}
