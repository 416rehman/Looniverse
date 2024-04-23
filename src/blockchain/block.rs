use crate::helpers::bytes_to_byte_string;
use chrono::Utc;
use log::info;
use serde::{Deserialize, Serialize};
use sha2::digest::Update;
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: i64,
    pub data: String,
    pub nonce: u64,
}

impl Block {
    /// Create and mine a new block
    pub fn new(id: u64, previous_hash: String, data: String, prefix: &String) -> Self {
        let timestamp = Utc::now().timestamp();
        let (nonce, hash) = Self::mine_block(id, timestamp, &previous_hash, &data, prefix);

        Self {
            id,
            nonce,
            data,
            timestamp,
            previous_hash,
            hash
        }
    }

    fn mine_block(
        id: u64,
        timestamp: i64,
        prev_hash: &String,
        data: &String,
        prefix: &String,
    ) -> (u64, String) {
        info!("Mining block!");
        let mut nonce = 0;

        loop {
            // Every 100000 iterations, print the current nonce.
            if nonce % 100000 == 0 {
                info!("Nonce: {}", nonce);
            }

            // Calculate the hash for the block.
            let hash = Self::generate_hash(&id, prev_hash, data, &timestamp, &nonce);

            // Check if the hash starts with the prefix
            let hash_byte_string = bytes_to_byte_string(&hash);
            if hash_byte_string.starts_with(prefix) {
                // Success! Print out and return.
                let hex_encoded_hash = hex::encode(&hash);
                info!(
                    "Mined! Nonce: {}, Hash: {}, Hash Byte String: {}",
                    nonce, hex_encoded_hash, hash_byte_string
                );
                return (nonce, hex_encoded_hash);
            }

            // Increase the nonce. Different nonce == different hash.
            nonce += 1;
        }
    }

    /// Serializes the block to a json string, then creates a SHA256 string of the json string.
    fn generate_hash(
        id: &u64,
        prev_hash: &String,
        data: &String,
        timestamp: &i64,
        nonce: &u64,
    ) -> Vec<u8> {
        let data = serde_json::json!({
            "id": id,
            "previous_hash": previous_hash,
            "data": data,
            "timestamp": timestamp,
            "nonce": nonce
        });

        let mut hasher = Sha256::new();
        hasher.update(data.to_string().as_bytes());

        hasher.finalize().as_slice().to_owned()
    }

    /// Makes sure the block is valid to be placed after the prev_block
    pub fn validate_block(&self, prev_block: &Block, prefix: &str) -> anyhow::Result<()> {
        // Make sure prev_block is the same as block's previous_block.
        if self.previous_hash != prev_block.hash {
            return Err(anyhow::anyhow!("Invalid block: previous hash is incorrect"));
        }

        // Make sure the hash starts with PREFIX
        if !bytes_to_byte_string(&hex::decode(&self.hash)?).starts_with(prefix) {
            return Err(anyhow::anyhow!("Block {} has invalid difficulty", self.id));
        }

        // Make sure the ID of the block is the next in line after previous_block's id.
        if self.id != prev_block.id + 1 {
            return Err(anyhow::anyhow!(
                "Block's id, {}, does not match the expected value: {} ({} + 1)",
                self.id,
                prev_block.id + 1,
                prev_block.id
            ));
        }

        // Calculate the new hash
        let calculated_hash = hex::encode(Block::generate_hash(
            &self.id,
            &self.previous_hash,
            &self.data,
            &self.timestamp,
            &self.nonce,
        ));
        if calculated_hash != self.hash {
            return Err(anyhow::anyhow!(
                "Block's previous hash {} is not the same as the calculated hash {}",
                self.previous_hash,
            ));
        }

        Ok(())
    }
}
