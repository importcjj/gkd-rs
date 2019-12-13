use crate::connection::Connection;
use crate::peer::Peer;
use crate::tunnel::Tunnel;
use crate::Result;
use async_std::net::TcpStream;
use async_std::net::{SocketAddr, ToSocketAddrs};
use async_std::task;

pub struct Client {
    local_addr: SocketAddr,
    peer: Peer,
}

impl Client {
    pub async fn connect<A: ToSocketAddrs>(remote: A, tunnel_num: u32) -> Result<Self> {
        let peer_id = 0;
        let peer = Peer::client_side(peer_id);
        let mut local_addr = None;
        for tunnel_id in 0..tunnel_num {
            let stream = TcpStream::connect(&remote).await?;
            if local_addr.is_none() {
                local_addr = Some(stream.local_addr()?)
            }
            let tunnel = Tunnel::client_side(peer_id, stream).await?;

            let inbound_sender = peer.inbound_sender.clone();
            let outbound = peer.outbound.clone();
            task::spawn(async move {
                if let Err(e) = tunnel.run(inbound_sender, outbound).await {
                    log::error!(
                        "Tunnel<{}> of Peer<{}> failed with {:?}",
                        tunnel_id,
                        peer_id,
                        e
                    );
                }
                log::info!("Tunnel<{}> of Peer<{}> closed", tunnel_id, peer_id);
            });

            log::info!("Tunnel<{}> of Peer<{}> established", tunnel_id, peer_id);
        }

        Ok(Client {
            peer,
            local_addr: local_addr.unwrap(),
        })
    }

    pub async fn get_connection(&self) -> Result<Connection> {
        self.peer.new_client_side_connection(self.local_addr).await
    }
}
