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
        loop {
            let readline = self.rl.readline(">> ");
            match readline {
                Ok(line) if line.trim().is_empty() => {
                    println!("Empty command, please try again.");
                    continue;
                }
                Ok(line) => {
                    self.rl.add_history_entry(line.as_str())?;
                    let line = line.trim();
                    match line.split_whitespace().nth(0).unwrap_or("") {
                        "\\q" => {
                            println!("Exiting CLI...");
                            break Ok(());
                        }
                        // TODO: Add more commands
                        _ => {
                            println!("Unknown command: {}", line);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    println!("Error reading line, please try again.");
                    break Err(e.into());
                }
            }
        }
    }
}