use crate::peer::{Peer, PeerGroup};
use crate::spawn_and_log_err;
use crate::tunnel::Tunnel;
use crate::Result;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::stream::StreamExt;
use async_std::sync::{Arc, Mutex, Weak};

pub struct Server {
    peers: Arc<Mutex<PeerGroup>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(PeerGroup::new())),
        }
    }

    pub async fn run_server(&self, addr: &str) -> Result<()> {
        let server = TcpListener::bind(addr).await?;
        while let Some(stream) = server.incoming().next().await {
            log::info!("new tunnel");
            let stream = stream?;
            let peers = Arc::downgrade(&self.peers);
            spawn_and_log_err(add_to_peer(peers, stream));
        }
        Ok(())
    }
}

async fn add_to_peer(peers: Weak<Mutex<PeerGroup>>, stream: TcpStream) -> Result<()> {
    let tunnel = Tunnel::server_side(stream).await?;

    let xpeers = match peers.upgrade() {
        Some(peers) => peers,
        None => return Ok(()),
    };

    let mut xpeers = xpeers.lock().await;
    let peer_id = tunnel.peer_id;
    let peer = xpeers
        .entry(peer_id)
        .or_insert_with(|| Peer::server_side(peer_id));

    let inbound_sender = peer.inbound_sender.clone();
    let outbound = peer.outbound.clone();
    spawn_and_log_err(tunnel.run_with_peer(peers, inbound_sender, outbound));
    Ok(())
}
