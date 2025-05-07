use crate::{block::Block, error::Result, transaction::Transaction, utxo_set::UTXOSet};
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
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

    fn remove_node(&self, addr: &str) -> Result<()> {
        let mut inner = self.inner.lock()?;
        if inner.known_nodes.contains(addr) {
            inner.known_nodes.remove(addr);
        }
        Ok(())
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

    fn send_data(&self, addr: &str, data: &[u8]) -> Result<()> {
        if addr == self.node_address {
            return Ok(());
        }
        let mut stream = match TcpStream::connect(addr) {
            Ok(stream) => stream,
            Err(_) => {
                self.remove_node(addr)?;
                return Ok(());
            }
        };

        stream.write(data)?;

        info!("data send successfully to {}", addr);
        Ok(())
    }

    fn send_addr(&self, addr: &str) -> Result<()> {
        info!("send addr to {}", addr);
        let nodes = self.get_known_nodes();
        let data = bincode::serialize(&(cmd_to_bytes("addr"), nodes))?;
        self.send_data(addr, &data)
    }

    fn get_known_nodes(&self) -> HashSet<String> {
        self.inner.lock().unwrap().known_nodes.clone()
    }
}

fn bytes_to_cmd(bytes: &[u8]) -> Result<Message> {
    let mut cmd = Vec::new();
    let cmd_bytes = &bytes[0..CMD_LEN];
    let data = &bytes[CMD_LEN..];
    for b in cmd_bytes {
        if 0 as u8 != *b {
            cmd.push(*b);
        }
    }
    info!("cmd: {}", String::from_utf8(&cmd)?);

    return match cmd {
        b"addr" => {
            let data: Vec<String> = deserialize(data)?;
            Ok(Message::Addr(data))
        }
        b"block" => {
            let data: Blockmsg = deserialize(data)?;
            Ok(Message::Block(data))
        }
        b"inv" => {
            let data: Invmsg = deserialize(data)?;
            Ok(Message::Inv(data))
        }
        b"getblocks" => {
            let data: GetBlocksmsg = deserialize(data)?;
            Ok(Message::GetBlock(data))
        }
        b"getdata" => {
            let data: GetDatamsg = deserialize(data)?;
            Ok(Message::GetData(data))
        }
        b"tx" => {
            let data: Txmsg = deserialize(data)?;
            Ok(Message::Tx(data))
        }
        b"version" => {
            let data: Versionmsg = deserialize(data)?;
            Ok(Message::Version(data))
        }
        _ => Err(format_err!("Unknown command in the server")),
    };
}

fn cmd_to_bytes(cmd: &str) -> [u8; CMD_LEN] {
    let mut data = [0; CMD_LEN];
    for (i, d) in cmd.as_bytes().iter().enumerate() {
        data[i] = *d;
    }
    data
}
