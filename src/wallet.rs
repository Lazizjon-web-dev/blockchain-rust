use crypto::{digest::Digest, ed25519, ripemd160::Ripemd160, sha2::Sha256};
use rand::{Rng, RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Wallet {
    pub secret_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl Wallet {
    fn new() -> Self {
        let mut key: [u8; 32] = [0; 32];
        OsRng.fill_bytes(&mut key);
        let (secret_key, public_key) = ed25519::keypair(&key);
        let secret_key = secret_key.to_vec();
        let public_key = public_key.to_vec();
        Wallet {
            secret_key,
            public_key,
        }
    }

    fn get_address(&self) -> String {
        let mut pub_hash = self.public_key.clone();
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