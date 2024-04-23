use crate::blockchain::block::Block;
use chrono::Utc;

const PREFIX: &str = "07";

pub struct Chain {
    pub blocks: Vec<Block>,
}

impl Chain {
    fn new() -> Self {
        Self { blocks: vec![] }
    }

    /// Inits the chain with a genesis block.
    fn init(&mut self) {
        let init_block = Block {
            id: 0,
            hash: "776165a3ae82c01e7862ecb5b26ee735a1c99c469426832f6a429c4909c48e5d".to_string(),
            previous_hash: "".to_string(),
            timestamp: Utc::now().timestamp(),
            data: "".to_string(),
            nonce: 1337,
        };

        self.blocks.push(init_block);
    }

    /// Add a block to the blockchain.
    /// Returns the ID of the newly added block
    fn try_add_block(&mut self, block: Block) -> anyhow::Result<()> {
        let last_block = self
            .blocks
            .last()
            .expect("Chain has no blocks. Make sure chain has been init()");
        block.validate_block(last_block, PREFIX)?;

        self.blocks.push(block);
        Ok(())
    }

    /// Validates all the block pairs in the chain
    fn validate_chain(&self) -> anyhow::Result<()> {
        for pair in self.blocks.windows(2) {
            let prev = &pair[0];
            let current = &pair[1];

            current.validate_block(prev, PREFIX)?;
        }

        Ok(())
    }

    /// Compares against a competing chain and returns the valid one, if both are valid, returns the chain with most blocks.
    fn get_authoritative_chain(self, competing_chain: Chain) -> anyhow::Result<Chain> {
        let is_local_valid = self.validate_chain();
        let is_remote_valid = competing_chain.validate_chain();

        // If both are valid, chose the longest chain.
        if is_local_valid && is_remote_valid {
            return if self.blocks.len() >= competing_chain.blocks.len() {
                Ok(self)
            } else {
                Ok(competing_chain)
            };
        }

        // Return the valid one of these 2, favoring remote.
        if is_remote_valid {
            return Ok(competing_chain);
        } else if is_local_valid {
            return Ok(self);
        }

        Err(anyhow::anyhow!("Both chains are invalid!"))
    }
}
