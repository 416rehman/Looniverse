pub mod blockchain;
mod helpers;
mod p2p;
use crate::blockchain::block::Block;
use crate::blockchain::ledger::Ledger;
use crate::p2p::{ChainResponse, LoonyBehaviourEvent};
use crate::p2p::{LocalChainRequest, LoonyBehaviour};
use libp2p::core::transport::upgrade;
use libp2p::floodsub::{FloodsubEvent, Topic};
use libp2p::futures::StreamExt;
use libp2p::mdns::Event;
use libp2p::swarm::SwarmEvent;
use libp2p::{identity, noise, tcp, PeerId, Transport};
use log::error;
use log::info;
use once_cell::sync::Lazy;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::UnboundedSender;

pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));

pub type LoonyEvent = SwarmEvent<LoonyBehaviourEvent>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id();
    info!("Peer Id: {}", peer_id.to_string());

    let (response_sender, mut response_rcv) = tokio::sync::mpsc::unbounded_channel();
    let (init_sender, mut init_rcv) = tokio::sync::mpsc::unbounded_channel();

    let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1)
        .authenticate(
            noise::Config::new(&keypair).expect("Signing libp2p-noise static DH keypair failed."),
        )
        .multiplex(libp2p::yamux::Config::default())
        .boxed();

    let behaviour =
        p2p::LoonyBehaviour::new(peer_id, vec![CHAIN_TOPIC.clone(), BLOCK_TOPIC.clone()]).await;

    let mut swarm = libp2p::swarm::Swarm::new(
        transport,
        behaviour,
        peer_id,
        libp2p::swarm::Config::with_tokio_executor(),
    );

    let mut stdin = BufReader::new(tokio::io::stdin()).lines();

    swarm
        .listen_on(
            "/ip4/0.0.0.0/tcp/0"
                .parse()
                .expect("should get a local socket"),
        )
        .expect("swarm should start");

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        info!("Sending init event!");
        init_sender.send(true).expect("channel should be up");
    });

    let mut ledger = Ledger::new();

    loop {
        let evt = {
            tokio::select! {
                line = stdin.next_line() => Some(p2p::EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                response = response_rcv.recv() => {
                    Some(p2p::EventType::LocalChainResponse(response.expect("response exists")))
                },
                data = init_rcv.recv() => {
                    if let Some(data) = data {
                        log::info!("init event received {:?}", data);
                        Some(p2p::EventType::Init)
                    } else {
                        // Channel closed or no more messages expected
                        None
                    }
                }
                event = swarm.select_next_some() => {
                    process_event(event, peer_id, &mut ledger, response_sender.clone(), swarm.behaviour_mut());
                    None
                },
            }
        };

        if let Some(event) = evt {
            match event {
                p2p::EventType::Init => {
                    let peers = p2p::get_list_peers(&swarm);
                    ledger.init();

                    info!("connected nodes: {}", peers.len());
                    if !peers.is_empty() {
                        let req = p2p::LocalChainRequest {
                            from_peer_id: peers
                                .iter()
                                .last()
                                .expect("at least one peer")
                                .to_string(),
                        };

                        let json = serde_json::to_string(&req).expect("can jsonify request");
                        let bytes = json.to_owned().into_bytes();
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(CHAIN_TOPIC.clone(), bytes);
                    }
                }
                p2p::EventType::LocalChainResponse(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    let bytes = json.to_owned().into_bytes();
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(CHAIN_TOPIC.clone(), bytes);
                }
                p2p::EventType::Input(line) => match line.as_str() {
                    "ls p" => p2p::handle_print_peers(&swarm),
                    cmd if cmd.starts_with("ls c") => ledger.print_chain(),
                    cmd if cmd.starts_with("create b") => p2p::handle_create_block(
                        cmd,
                        &blockchain::ledger::PREFIX.to_string(),
                        &mut ledger.chain,
                        BLOCK_TOPIC.clone(),
                        &mut swarm,
                    ),
                    _ => log::error!("unknown command"),
                },
            }
        }
    }
}

fn process_event(
    event: LoonyEvent,
    peer_id: PeerId,
    ledger: &mut Ledger,
    response_sender: UnboundedSender<ChainResponse>,
    loony_behaviour: &mut LoonyBehaviour,
) {
    match event {
        LoonyEvent::Behaviour(idk) => match idk {
            LoonyBehaviourEvent::Floodsub(flood_event) => match flood_event {
                FloodsubEvent::Message(msg) => {
                    if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data) {
                        if resp.receiver == peer_id.to_string() {
                            info!("Response from {}:", msg.source);
                            resp.chain.iter().for_each(|r| info!("{:?}", r));

                            let _ = ledger.update_to_authoritative_chain(resp.chain);
                        }
                    } else if let Ok(resp) = serde_json::from_slice::<LocalChainRequest>(&msg.data)
                    {
                        info!("sending local chain to {}", msg.source.to_string());
                        if peer_id.to_string() == resp.from_peer_id {
                            if let Err(e) = response_sender.send(ChainResponse {
                                chain: ledger.chain.clone(),
                                receiver: msg.source.to_string(),
                            }) {
                                error!("error sending response via channel, {}", e);
                            }
                        }
                    } else if let Ok(block) = serde_json::from_slice::<Block>(&msg.data) {
                        info!("received new block from {}", msg.source.to_string());
                        let _ = ledger.try_add_block(block);
                    }
                }
                FloodsubEvent::Subscribed { peer_id, topic } => {
                    info!("Peer {:?} subscribed to topic: {:?}", peer_id, topic);
                }
                FloodsubEvent::Unsubscribed { peer_id, topic } => {
                    info!("Peer {:?} unsubscribed from topic: {:?}", peer_id, topic);
                }
            },
            LoonyBehaviourEvent::Mdns(mdns_event) => match mdns_event {
                Event::Discovered(discovered_list) => {
                    for (peer, _addr) in discovered_list {
                        loony_behaviour.floodsub.add_node_to_partial_view(peer);
                    }
                }
                Event::Expired(expired_list) => {
                    for (peer, _addr) in expired_list {
                        if !loony_behaviour.mdns.discovered_nodes().any(|&p| p == peer) {
                            loony_behaviour
                                .floodsub
                                .remove_node_from_partial_view(&peer);
                        }
                    }
                }
            },
        },

        _ => {}
    }
}
