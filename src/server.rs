use crate::Result;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::stream::StreamExt;
use crate::spawn_and_log_err;
use async_std::sync::{Mutex, MutexGuard, Arc, Weak};
use crate::peer::{PeerGroup, Peer};
use crate::tunnel::Tunnel;

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
            let stream = stream?;
            let peers = Arc::downgrade(&self.peers);
            spawn_and_log_err(add_to_peer(peers, stream));
        }
        Ok(())
    }
}

async fn add_to_peer(peers: Weak<Mutex<PeerGroup>>, stream: TcpStream) -> Result<()> {
    let tunnel = Tunnel::new_from_tcp_stream(stream).await?;
    
    let peers = match peers.upgrade() {
        Some(peers) => peers,
        None => return Ok(())
    };

    let mut peers = peers.lock().await;
    let peer_id = tunnel.peer_id;
    let peer = peers.entry(peer_id).or_insert(Peer::new(peer_id));

    let inbound_sender = peer.inbound_sender.clone();
    let outbound = peer.outbound.clone();
    spawn_and_log_err(tunnel.run(inbound_sender, outbound));
    Ok(())
}