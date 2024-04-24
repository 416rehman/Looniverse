use std::collections::HashSet;
use crate::blockchain::block::Block;
use libp2p::floodsub::{Floodsub, Topic};
use libp2p::mdns::Config;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{PeerId, Swarm};
use log::info;
use serde::{Deserialize, Serialize};

/// Used to send our local chain to remote peer(s).
#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub chain: Vec<Block>,
    pub receiver: String,
}

/// Used to request a chain from a remote peer.
#[derive(Debug, Serialize, Deserialize)]
pub struct LocalChainRequest {
    pub from_peer_id: String,
}

/// Types of events used across the application. Could be lifted out??
pub enum EventType {
    LocalChainResponse(ChainResponse),
    Input(String),
    Init,
}

#[derive(NetworkBehaviour)]
pub struct LoonyBehaviour {
    pub floodsub: Floodsub,
    // Used to find nodes in LOCAL network only, not outside.
    pub mdns: libp2p::mdns::tokio::Behaviour,
}

impl LoonyBehaviour {
    pub async fn new(
        id: PeerId,
        topics: Vec<Topic>
    ) -> Self {
        // Create the floodsub and subscribe to interested topics.
        let mut floodsub = Floodsub::new(id);
        for topic in topics {
            floodsub.subscribe(topic);
        }

        // Create and return the LoonyBehaviour
        Self {
            floodsub,
            mdns: libp2p::mdns::tokio::Behaviour::new(Config::default(), id).expect("MDNS should be created.")
        }
    }
}

pub fn get_list_peers(swarm: &Swarm<LoonyBehaviour>) -> Vec<String> {
    info!("Discovered Peers:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub fn handle_print_peers(swarm: &Swarm<LoonyBehaviour>) {
    let peers = get_list_peers(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn handle_create_block(cmd: &str, prefix: &String, chain: &mut Vec<Block>, topic: Topic, swarm: &mut Swarm<LoonyBehaviour>) {
    if let Some(data) = cmd.strip_prefix("create b") {
        let behaviour = swarm.behaviour_mut();
        let latest_block = chain
            .last()
            .expect("there is at least one block");

        let block = Block::new(
            latest_block.id + 1,
            latest_block.hash.clone(),
            data.to_owned(),
            prefix
        );

        let json = serde_json::to_string(&block).expect("can jsonify request");
        let bytes = json.as_bytes().to_vec();
        chain.push(block);
        info!("broadcasting new block");
        behaviour
            .floodsub
            .publish(topic, bytes);
    }
}
