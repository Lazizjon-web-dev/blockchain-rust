use super::*;
use bincode::{deserialize, serialize};
use bitcoincash_addr::{Address, HashType, Scheme};
use crypto::{digest::Digest, ed25519, ripemd160::Ripemd160, sha2::Sha256};
use log::info;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Wallet {
    pub secret_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl Wallet {
    fn new() -> Self {
        let mut key: [u8; 32] = [0; 32];
        thread_rng().fill_bytes(&mut key);
        let (secret_key, public_key) = ed25519::keypair(&key);
        let secret_key = secret_key.to_vec();
        let public_key = public_key.to_vec();
        Wallet {
            secret_key,
            public_key,
        }
    }

    fn get_address(&self) -> String {
        let mut pub_hash: Vec<u8> = self.public_key.clone();
        hash_pub_key(&mut pub_hash);
        let address = Address {
            body: pub_hash,
            scheme: Scheme::Base58,
            hash_type: HashType::Script,
            ..Default::default()
        };
        // 0 O 1 I
        address.encode().unwrap()
    }
}

pub fn hash_pub_key(pub_key: &mut Vec<u8>) {
    let mut hasher1 = Sha256::new();
    hasher1.input(pub_key);
    hasher1.result(pub_key);
    let mut hasher2 = Ripemd160::new();
    hasher2.input(pub_key);
    pub_key.resize(20, 0);
    hasher2.result(pub_key);
}

pub struct Wallets {
    wallets: HashMap<String, Wallet>,
}

impl Wallets {
    pub fn new() -> Result<Self> {
        let mut wlt = Wallets {
            wallets: HashMap::<String, Wallet>::new(),
        };

        let db = sled::open("data/wallets")?;
        for item in db.into_iter() {
            let i = item?;
            let address = String::from_utf8(i.0.to_vec())?;
            let wallet = deserialize(&i.1.to_vec())?;
            wlt.wallets.insert(address, wallet);
        }
        drop(db);
        Ok(wlt)
    }

    pub fn create_wallet(&mut self) -> String {
        let wallet = Wallet::new();
        let address = wallet.get_address();
        self.wallets.insert(address.clone(), wallet);
        info!("create wallet: {}", address);
        address
    }

    pub fn get_all_addresses(&self) -> Vec<String> {
        let mut addresses = Vec::new();
        for (address, _) in &self.wallets {
            addresses.push(address.clone());
        }
        addresses
    }

    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        self.wallets.get(address)
    }

    pub fn save_all(&self) -> Result<()> {
        let db = sled::open("data/wallets")?;

        for (address, wallet) in &self.wallets {
            let data = serialize(wallet)?;
            db.insert(address, data)?;
        }

        db.flush()?;
        drop(db);
        Ok(())
    }
}
