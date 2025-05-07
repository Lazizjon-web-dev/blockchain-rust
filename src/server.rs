use crate::{block::Block, error::Result, transaction::Transaction, utxo_set::UTXOSet};
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    net::{TcpListener, TcpStream},
};

const KNOWN_NODE1: &str = "localhost: 3000";
const CMD_LEN: usize = 12;
const VERSION: i32 = 1;

pub struct Server {
    node_address: String,
    mining_address: String,
    inner: Arch<Mutex<ServerInner>>,
}

struct ServerInner {
    known_nodes: HashSet<String>,
    utxo: UTXOSet,
    blocks_in_transit: Vec<String>,
    mempool: HashMap<String, Transaction>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BlockMsg {
    address_from: String,
    block: Block,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GetBlocksMsg {
    address_from: String,
}

struct GetDataMsg {
    address_from: String,
    kind: String,
    id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InviteMsg {
    address_from: String,
    kind: String,
    items: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TransactionMsg {
    address_from: String,
    transaction: Transaction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VersionMsg {
    address_from: String,
    version: i32,
    best_height: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Message {
    Address(Vec<String>),
    Version(VersionMsg),
    Transaction(TransactionMsg),
    GetData(GetDataMsg),
    GetBlocks(GetBlocksMsg),
    Invite(InviteMsg),
    Block(BlockMsg),
}

impl Server {
    pub fn new(port: &str, miner_address: &str, utxo: UTXOSet) -> Result<Self> {
        let mut node_set = HashSet::new();
        node_set.insert(String::from(KNOWN_NODE1));
        Ok(Self {
            node_address: String::from(format!("localhost:{}", port)),
            mining_address: miner_address.to_string(),
            inner: Arc::new(Mutex::new(ServerInner {
                known_nodes: node_set,
                utxo,
                blocks_in_transit: Vec::new(),
                mempool: HashMap::new(),
            })),
        })
    }

    fn handle_connection(&self, mut stream: TcpStream) -> Result<()> {
        let mut buffer = Vec::new();
        let count = stream.read_to_end(&mut buffer)?;
        info!("Accept request: length {}", count);

        let cmd = bytes_to_cmd(&buffer)?;

        match cmd {
            Message::Address(data) => self.handle_address(data)?,
            Message::Block(data) => self.handle_block(data)?,
            Message::Invite(data) => self.handle_invite(data)?,
            Message::GetBlocks(data) => self.handle_get_blocks(data)?,
            Message::GetData(data) => self.handle_get_data(data)?,
            Message::Transaction(data) => self.handle_transaction(data)?,
            Message::Version(data) => self.handle_version(data)?,
        };
        Ok(())
    }
}