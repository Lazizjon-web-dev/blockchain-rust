use crate::{
    blockchain::Blockchain,
    error::Result,
    server::Server,
    transaction::Transaction,
    utxoset::UTXOSet,
    wallets::{Wallet, Wallets},
};
use bitcoincash_addr::Address;
use clap::{arg, Command};
use std::process::exit;

pub struct Cli {}

impl Cli {
    pub fn new() -> Result<Self> {
        Ok(Cli {})
    }
    pub fn run(&mut self) -> Result<()> {
        let matches = Command::new("blockchain-rust")
            .version("0.1")
            .author("Lazizjon-web-dev")
            .about("A simple CLI for interacting with a blockchain")
            .subcommand(Command::new("print").about("Print the blockchain"))
            .subcommand(Command::new("create_wallet").about("Create a new wallet"))
            .subcommand(Command::new("list_addresses").about("List all addresses"))
            .subcommand(Command::new("reindex").about("Reindex the UTXO set"))
            .subcommand(
                Command::new("getbalance")
                    .about("Get the balance of an address")
                    .arg(arg!(<address> "'The address to get the balance of'")),
            )
            .subcommand(
                Command::new("startnode")
                    .about("Start the node server")
                    .arg(arg!(<PORT>"'the port server bind to locally'")),
            )
            .subcommand(
                Command::new("create")
                    .about("Create a new blockchain")
                    .arg(arg!(<ADDRESS>"'The address to send genesis block reward to' ")),
            )
            .subcommand(
                Command::new("send")
                    .about("send coins in the blockchain")
                    .arg(arg!(<FROM>" 'Source wallet address'"))
                    .arg(arg!(<TO>" 'Destination wallet address'"))
                    .arg(arg!(<AMOUNT>" 'Amount to send'"))
                    .arg(arg!(-m --mine " 'the from address mine immidiately'")),
            )
            .subcommand(
                Command::new("startminer")
                    .about("Start the miner server")
                    .arg(arg!(<PORT>"'the port server bind to locally'"))
                    .arg(arg!(<ADDRESS>"'wallet address'")),
            )
            .get_matches();

        if let Some(ref matches) = matches.subcommand_matches("startminer") {
            let port = if let Some(port) = matches.get_one::<String>("PORT") {
                port
            } else {
                println!("PORT not supply!: usage");
                exit(1)
            };

            let address = if let Some(address) = matches.get_one::<String>("ADDRESS") {
                address
            } else {
                println!("ADDRESS not supply!: usage");
                exit(1)
            };

            let blockchain = Blockchain::new()?;
            let utxo_set = UTXOSet { blockchain };
            let server = Server::new(port, address, utxo_set)?;
            server.start()?;
        }

        if let Some(_) = matches.subcommand_matches("create_wallet") {
            println!("address: {}", cmd_create_wallet()?);
        }

        if let Some(_) = matches.subcommand_matches("list_addresses") {
            cmd_list_addresses()?;
        }

        if let Some(_) = matches.subcommand_matches("reindex") {
            let count = cmd_reindex()?;
            println!("Done! There are {} transactions in the UTXO set", count);
        }

        if let Some(ref matches) = matches.subcommand_matches("create") {
            if let Some(address) = matches.get_one::<String>("ADDRESS") {
                cmd_create_blockchain(address);
            }
        }

        if let Some(ref matches) = matches.subcommand_matches("getbalance") {
            if let Some(address) = matches.get_one::<String>("ADDRESS") {
                let balance = cmd_get_balance(address)?;
                println!("Balance: {}\n", balance);
            }
        }

        if let Some(ref matches) = matches.subcommand_matches("startnode") {
            if let Some(port) = matches.get_one::<String>("PORT") {
                let blockchain = Blockchain::new()?;
                let utxo_set = UTXOSet { blockchain };
                let server = Server::new(port, "", utxo_set)?;
                server.start()?;
            }
        }

        if let Some(ref matches) = matches.subcommand_matches("send") {
            let from = if let Some(address) = matches.get_one::<String>("FROM") {
                address
            } else {
                println!("from not supply!: usage");
                exit(1)
            };

            let to = if let Some(address) = matches.get_one::<String>("TO") {
                address
            } else {
                println!("to not supply!: usage");
                exit(1)
            };

            let amount: i32 = if let Some(amount) = matches.get_one::<String>("AMOUNT") {
                amount.parse()?
            } else {
                println!("amount not supply!: usage");
                exit(1)
            };

            cmd_send(from, to, amount, matches.contains_id("mine"))?;
        }

        if let Some(_) = matches.subcommand_matches("print") {
            cmd_print_chain()?;
        }

        Ok(())
    }
}

fn cmd_send(from: &str, to: &str, amount: i32, mine_now: bool) -> Result<()> {
    let blockchain = Blockchain::new()?;
    let mut utxo_set = UTXOSet { blockchain };
    let wallets = Wallets::new()?;
    let wallet = wallets.get_wallet(from).unwrap();
    let transaction = Transaction::new_UTXO(wallet, to, amount, &utxo_set)?;
    if mine_now {
        let cbtx = Transaction::new_coinbase(from.to_string(), String::from("Reward"))?;
        let new_block = utxo_set.blockchain.mine_block(vec![cbtx, transaction])?;
        utxo_set.update(&new_block)?;
    } else {
        Server::send_transaction(&transaction, utxo_set)?;
    }

    println!("Success! Transaction sent");
    Ok(())
}

fn cmd_create_wallet() -> Result<String> {
    let mut wallets = Wallets::new()?;
    let address = wallets.create_wallet();
    wallets.save_all()?;
    Ok(address)
}

fn cmd_reindex() -> Result<i32> {
    let blockchain = Blockchain::new()?;
    let utxo_set = UTXOSet { blockchain };
    utxo_set.reindex()?;
    utxo_set.count_transactions()
}

fn cmd_list_addresses() -> Result<()> {
    let wallets = Wallets::new()?;
    let addresses = wallets.get_all_addresses();
    println!("addresses: ");
    for address in addresses {
        println!("{}", address);
    }
    Ok(())
}

fn cmd_create_blockchain(address: &str) -> Result<()> {
    let address = String::from(address);
    let blockchain = Blockchain::create_blockchain(address)?;

    let utxo_set = UTXOSet { blockchain };
    utxo_set.reindex()?;
    println!("create blockchain");
    Ok(())
}

fn cmd_get_balance(address: &str) -> Result<i32> {
    let pub_key_hash = Address::decode(address).unwrap().body;
    let blockchain = Blockchain::new()?;
    let utxo_set = UTXOSet { blockchain };
    let utxos = utxo_set.find_UTXO(&pub_key_hash)?;

    let mut balance = 0;
    for output in utxos.outputs {
        balance += output.value;
    }
    Ok(balance)
}

fn cmd_print_chain() -> Result<()> {
    let blockchain = Blockchain::new()?;
    for block in blockchain.iter() {
        println!("{:#?}", block);
    }
    Ok(())
}
