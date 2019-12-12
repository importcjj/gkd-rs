use crate::connection::Connection;
use crate::peer::{Peer, PeerGroup};
use crate::spawn_and_log_err;
use crate::tunnel::Tunnel;
use crate::Result;
use async_std::io;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::net::{SocketAddr, ToSocketAddrs};
use async_std::stream::StreamExt;
use async_std::sync::{channel, Arc, Mutex, Receiver, Sender, Weak};

pub struct Server {
    local_addr: SocketAddr,
    shares: Arc<Mutex<Shares>>,
    incomings: Receiver<(Connection, SocketAddr)>,
}

pub struct Shares {
    pub peers: PeerGroup,
    to_incomings: Sender<(Connection, SocketAddr)>,
}

impl Server {
    pub async fn bind<A: ToSocketAddrs>(addrs: A) -> Result<Self> {
        let listener = TcpListener::bind(addrs).await?;
        let local_addr = listener.local_addr().unwrap();
        let peers = PeerGroup::new();
        let (to_incomings, incomings) = channel(1024);

        let shares = Arc::new(Mutex::new(Shares {
            peers,
            to_incomings,
        }));

        let server = Self {
            local_addr,
            incomings,
            shares,
        };

        let peers = Arc::downgrade(&server.shares);
        spawn_and_log_err(async move {
            while let Some(stream) = listener.incoming().next().await {
                log::info!("new tunnel");
                let stream = stream?;
                let peers = peers.clone();

                spawn_and_log_err(add_to_peer(peers, stream));
            }

            Ok::<(), io::Error>(())
        });

        Ok(server)
    }

    pub async fn accept(&self) -> Option<(Connection, SocketAddr)> {
        self.incomings.recv().await
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.local_addr)
    }
}

async fn add_to_peer(shares: Weak<Mutex<Shares>>, stream: TcpStream) -> Result<()> {
    let tunnel = Tunnel::server_side(stream).await?;

    let shares_arc = match shares.upgrade() {
        Some(peers) => peers,
        None => return Ok(()),
    };

    let mut shares_guard = shares_arc.lock().await;
    let peer_id = tunnel.peer_id;
    let to_conn_incomings = shares_guard.to_incomings.clone();
    let peer = shares_guard
        .peers
        .entry(peer_id)
        .or_insert_with(|| Peer::server_side(peer_id, to_conn_incomings));

    let inbound_sender = peer.inbound_sender.clone();
    let outbound = peer.outbound.clone();
    spawn_and_log_err(tunnel.run_with_shares(shares, inbound_sender, outbound));
    Ok(())
}
