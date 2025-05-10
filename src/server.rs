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

    pub fn send_tx(&self, addr: &str, tx: &Transaction) -> Result<()> {
        info!("send tx to: {}  txid: {}", addr, &tx.id);
        let data = TransactionMsg {
            address_from: self.node_address.clone(),
            transaction: tx.clone(),
        };
        let data = bincode::serialize(&(cmd_to_bytes("tx"), data))?;
        self.send_data(addr, &data)
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

    fn send_get_blocks(&self, addr: &str) -> Result<()> {
        info!("send get blocks message to {}", addr);
        let data = GetBlocksMsg {
            address_from: self.node_address.clone(),
        };
        let data = bincode::serialize(&(cmd_to_bytes("getblocks"), data))?;
        self.send_data(addr, &data)
    }

    fn send_get_data(&self, addr: &str, kind: &str, id: &str) -> Result<()> {
        info!(
            "send get data message to {} kind: {} id: {}",
            addr, kind, id
        );
        let data = GetDataMsg {
            address_from: self.node_address.clone(),
            kind: kind.to_string(),
            id: id.to_string(),
        };
        let data = bincode::serialize(&(cmd_to_bytes("getdata"), data))?;
        self.send_data(addr, &data)
    }

    fn send_block(&self, addr: &str, block: &Block) -> Result<()> {
        info!("send block to {} block hash: {}", addr, block.get_hash());
        let data = BlockMsg {
            address_from: self.node_address.clone(),
            block: block.clone(),
        };
        let data = bincode::serialize(&(cmd_to_bytes("block"), data))?;
        self.send_data(addr, &data)
    }

    fn send_inv(&self, addr: &str, kind: &str, items: Vec<String>) -> Result<()> {
        info!(
            "send inv message to {} kind: {} data: {:?}",
            addr, kind, items
        );
        let data = InviteMsg {
            address_from: self.node_address.clone(),
            kind: kind.to_string(),
            items,
        };
        let data = bincode::serialize(&(cmd_to_bytes("inv"), data))?;
        self.send_data(addr, &data)
    }

    fn send_version(&self, addr: &str) -> Result<()> {
        info!("send version info to: {}", addr);
        let data = VersionMsg {
            address_from: self.node_address.clone(),
            best_height: self.get_best_height(),
            version: VERSION,
        };
        let data = bincode::serialize(&(cmd_to_bytes("version"), data))?;
        self.send_data(addr, &data)
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

    fn  handle_address(&self, msg: Vec<String>) -> Result<()> {
        info("recieved address message: {:#?}", msg);
        for node in msg {
            self.add_nodes(&node)?;
        }
        Ok(())
    }

    fn handle_block(&self, msg: BlockMsg) -> Result<()> {
        info!("recieved block message: {}, {}", msg.address_from, msg.block.get_hash());
        self.add_block(msg.block)?;

        let mut in_transit = self.get_in_transit();
        if in_transit.len() > 0 {
            let block_hash = &in_transit[0];
            self.send_get_data(&msg.address_from, "block", block_hash)?;
            in_transit.remove(0);
            self.replace_in_transit(in_transit);
        } else {
            self.utxo_reindex()?;
        }
        Ok(())
    }

    fn handle_get_blocks(&self, msg: GetBlocksMsg) -> Result<()> {
        info!("recieved get blocks message: {:#?}", msg);
        let block_hashes = self.get_block_hashes();
        self.send_inv(&msg.address_from, "block", block_hashes)?;
        Ok(())
    }

    fn handle_get_data(&self, msg: GetDataMsg) -> Result<()> {
        info!("recieved get data message: {:#?}", msg);
        match msg.kind.as_str() {
            "block" => {
                let block = self.get_block(&msg.id)?;
                self.send_block(&msg.address_from, &block)?;
            }
            "tx" => {
                let tx = self.get_mempool_tx(&msg.id).unwrap();
                self.send_tx(&msg.address_from, &tx)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn get_block_hashes(&self) -> Vec<String> {
        self.inner.lock().unwrap().utxo.blockchain.get_block_hashes()
    }

    fn add_nodes(&self, addr: &str) {
        self.inner.lock().unwrap().known_nodes.insert(String::from(addr));
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
