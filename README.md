# Looniverse Blockchain

Looniverse is a simple blockchain implementation written in Rust. It provides the basic functionality of a blockchain, including block creation, mining, validation, and chain management.

## Features

- Block creation and mining
- Chain validation
- Competing chain resolution

## Getting Started

To get started with Looniverse, follow these steps:

1. Clone the repository:

   ```bash
   git clone https://github.com/416rehman/looniverse.git
   ```

2. Navigate to the project directory:

   ```bash
   cd looniverse
   ```

3. Build the project:

   ```bash
   cargo build
   ```

## Usage

Below is an example of how to use the Looniverse blockchain:

```rust
// Create a new blockchain
let mut chain = Chain::new();

// Initialize the chain with a genesis block
chain.init();

// Add a new block to the chain
let new_block = Block::new(
    chain.blocks.len() as u64,
    chain.blocks.last().unwrap().hash.clone(),
    "Some data for the new block".to_string(),
    &PREFIX.to_string(),
);
chain.try_add_block(new_block)?;

// Validate the entire chain
chain.validate_chain()?;
```
