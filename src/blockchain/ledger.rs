use crate::blockchain::block::Block;
use chrono::Utc;
use log::info;
use serde::{Deserialize, Serialize};

pub const PREFIX: &str = "0000000";

#[derive(Debug, Serialize, Deserialize)]
pub struct Ledger {
    pub chain: Vec<Block>,
}

impl Ledger {
    pub fn new() -> Self {
        Self { chain: vec![] }
    }

    /// Inits the chain with a genesis block.
    pub fn init(&mut self) {
        let init_block = Block {
            id: 0,
            hash: "776165a3ae82c01e7862ecb5b26ee735a1c99c469426832f6a429c4909c48e5d".to_string(),
            previous_hash: "".to_string(),
            timestamp: Utc::now().timestamp(),
            data: "".to_string(),
            nonce: 1337,
        };

        self.chain.push(init_block);
    }

    /// Add a block to the blockchain.
    /// Returns the ID of the newly added block
    pub fn try_add_block(&mut self, block: Block) -> anyhow::Result<()> {
        let last_block = self
            .chain
            .last()
            .expect("Chain has no blocks. Make sure chain has been init()");
        block.validate_block(last_block, PREFIX)?;

        self.chain.push(block);
        Ok(())
    }

    /// Validates all the block pairs in the chain
    fn validate_chain(chain: &Vec<Block>) -> anyhow::Result<()> {
        for pair in chain.windows(2) {
            let prev = &pair[0];
            let current = &pair[1];

            current.validate_block(prev, PREFIX)?;
        }

        Ok(())
    }

    pub fn print_chain(&self) {
        info!("Local Blockchain:");
        let pretty_json = serde_json::to_string_pretty(&self.chain).expect("can jsonify blocks");
        info!("{}", pretty_json);
    }

    /// Compares against a competing chain and returns the valid one, if both are valid, returns the chain with most blocks.
    pub fn update_to_authoritative_chain(
        &mut self,
        competing_chain: Vec<Block>,
    ) -> anyhow::Result<()> {
        let is_local_valid = Self::validate_chain(&self.chain);
        let is_remote_valid = Self::validate_chain(&competing_chain);

        // If both are valid, chose the longest chain.
        if is_local_valid.is_ok() && is_remote_valid.is_ok() {
            if self.chain.len() >= competing_chain.len() {
                return Ok(());
            } else {
                self.chain = competing_chain;
                return Ok(());
            };
        }

        // Return the valid one of these 2, favoring remote.
        if is_remote_valid.is_ok() {
            self.chain = competing_chain;
            return Ok(());
        } else if is_local_valid.is_ok() {
            return Ok(());
        }

        Err(anyhow::anyhow!("Both chains are invalid!"))
    }
}
