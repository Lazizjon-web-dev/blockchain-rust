use crate::{block::Block, error::Result, server, transaction::Transaction, utxoset::UTXOSet};
use core::time::Duration;
use failure::format_err;
use log::{info, debug};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    net::{TcpListener, TcpStream},
    io::{Read, Write},
    thread,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

    pub fn start(&self) -> Result<()> {
        let server1 = Self {
            node_address: self.node_address.clone(),
            mining_address: self.mining_address.clone(),
            inner: Arc::clone(&self.inner),
        };
        info!(
            "Starting server at {}, mining address: {}",
            self.node_address, &self.mining_address
        );

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1000));
            if server1.get_best_height()? == -1 {
                server1.request_blocks()?;
            } else {
                server1.send_version(KNOWN_NODE1)?;
            }
        });

        let listener = TcpListener::bind(&self.node_address)?;
        info!("Server listen...");

        for stream in listener.incoming() {
            let stream = stream?;
            let server1 = Self {
                node_address: self.node_address.clone(),
                mining_address: self.mining_address.clone(),
                inner: Arc::clone(&self.inner),
            };
            thread::spawn(move || server1.handle_connection(stream));
        }

        Ok(())
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

    fn request_blocks(&self) -> Result<()> {
        for node in self.get_known_nodes() {
            self.send_get_blocks(&node)?
        }
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
            best_height: self.get_best_height()?,
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

    fn handle_address(&self, msg: Vec<String>) -> Result<()> {
        info!("recieved address message: {:#?}", msg);
        for node in msg {
            self.add_nodes(&node);
        }
        Ok(())
    }

    fn handle_block(&self, msg: BlockMsg) -> Result<()> {
        info!(
            "recieved block message: {}, {}",
            msg.address_from,
            msg.block.get_hash()
        );
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
    //TODO: refactor this function to make it shorter and more readable
    fn handle_transaction(&self, msg: TransactionMsg) -> Result<()> {
        info!("recieved transaction message: {:#?}", msg);
        self.insert_mempool(msg.transaction.clone());

        let known_nodes = self.get_known_nodes();

        if self.node_address == KNOWN_NODE1 {
            for node in known_nodes {
                if node != self.node_address && node != msg.address_from {
                    self.send_inv(&node, "tx", vec![msg.transaction.id.clone()])?;
                }
            }
        } else {
            let mut mempool = self.get_mempool();
            debug!("Current mempool: {:#?}", &mempool);
            if mempool.len() >= 1 && !self.mining_address.is_empty() {
                loop {
                    let mut txs = Vec::new();

                    for (_, tx) in &mempool {
                        if self.verify_tx(tx)? {
                            txs.push(tx.clone());
                        }
                    }

                    if txs.is_empty() {
                        return Ok(());
                    }

                    let cbtx =
                        Transaction::new_coinbase(self.mining_address.clone(), String::new())?;
                    txs.push(cbtx);

                    for tx in &txs {
                        mempool.remove(&tx.id);
                    }

                    let new_block = self.mine_block(txs)?;
                    self.utxo_reindex()?;

                    for node in self.get_known_nodes() {
                        if node != self.node_address {
                            self.send_inv(&node, "block", vec![new_block.get_hash()])?;
                        }
                    }

                    if mempool.len() == 0 {
                        break;
                    }
                }

                self.clear_mempool();
            }
        }

        Ok(())
    }

    fn handle_invite(&self, msg: InviteMsg) -> Result<()> {
        info!("recieved invite message: {:#?}", msg);
        if msg.kind == "block" {
            let block_hash = &msg.items[0];
            self.send_get_data(&msg.address_from, "block", block_hash)?;

            let mut new_in_transit = Vec::new();
            for b in &msg.items {
                if b != block_hash {
                    new_in_transit.push(b.clone());
                }
            }
            self.replace_in_transit(new_in_transit);
        } else if msg.kind == "tx" {
            let tx_id = &msg.items[0];
            match self.get_mempool_tx(tx_id) {
                Some(tx) => {
                    if tx.id.is_empty() {
                        self.send_get_data(&msg.address_from, "tx", tx_id)?;
                    }
                }
                None => self.send_get_data(&msg.address_from, "tx", tx_id)?,
            }
        }
        Ok(())
    }

    fn add_block(&self, block: Block) -> Result<()> {
        self.inner.lock().unwrap().utxo.blockchain.add_block(block)
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

    fn get_block(&self, block_hash: &str) -> Result<Block> {
        self.inner
            .lock()
            .unwrap()
            .utxo
            .blockchain
            .get_block(block_hash)
    }

    fn handle_version(&self, msg: VersionMsg) -> Result<()> {
        info!("recieved version message: {:#?}", msg);
        let my_best_height = self.get_best_height();
        if my_best_height < msg.best_height {
            self.send_get_blocks(&msg.address_from)?;
        } else if my_best_height > msg.best_height {
            self.send_version(&msg.address_from)?;
        }

        self.send_addr(&msg.address_from)?;

        if !self.node_is_known(&msg.address_from) {
            self.add_nodes(&msg.address_from);
        }
        Ok(())
    }

    fn get_best_height(&self) -> Result<i32> {
        self.inner.lock().unwrap().utxo.blockchain.get_best_height()
    }

    fn get_block_hashes(&self) -> Vec<String> {
        self.inner
            .lock()
            .unwrap()
            .utxo
            .blockchain
            .get_block_hashes()
    }

    fn node_is_known(&self, addr: &str) -> bool {
        self.inner.lock().unwrap().known_nodes.get(addr).is_some()
    }

    fn add_nodes(&self, addr: &str) {
        self.inner
            .lock()
            .unwrap()
            .known_nodes
            .insert(String::from(addr));
    }

    fn replace_in_transit(&self, hashs: Vec<String>) {
        let bit = &mut self.inner.lock().unwrap().blocks_in_transit;
        bit.clone_from(&hashs);
    }

    fn get_in_transit(&self) -> Vec<String> {
        self.inner.lock().unwrap().blocks_in_transit.clone()
    }

    fn get_mempool_tx(&self, addr: &str) -> Option<Transaction> {
        match self.inner.lock().unwrap().mempool.get(addr) {
            Some(tx) => Some(tx.clone()),
            None => None,
        }
    }

    fn get_mempool(&self) -> HashMap<String, Transaction> {
        self.inner.lock().unwrap().mempool.clone()
    }

    fn insert_mempool(&self, tx: Transaction) {
        self.inner.lock().unwrap().mempool.insert(tx.id.clone(), tx);
    }

    fn clear_mempool(&self) {
        self.inner.lock().unwrap().mempool.clear()
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
            Ok(Message::Address(data))
        }
        b"block" => {
            let data: BlockMsg = deserialize(data)?;
            Ok(Message::Block(data))
        }
        b"inv" => {
            let data: InviteMsg = deserialize(data)?;
            Ok(Message::Invite(data))
        }
        b"getblocks" => {
            let data: GetBlocksMsg = deserialize(data)?;
            Ok(Message::GetBlocks(data))
        }
        b"getdata" => {
            let data: GetDataMsg = deserialize(data)?;
            Ok(Message::GetData(data))
        }
        b"tx" => {
            let data: TransactionMsg = deserialize(data)?;
            Ok(Message::Transaction(data))
        }
        b"version" => {
            let data: VersionMsg = deserialize(data)?;
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
