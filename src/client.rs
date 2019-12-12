use crate::connection::Connection;
use crate::peer::Peer;
use crate::spawn_and_log_err;
use crate::tunnel::Tunnel;
use crate::Result;
use async_std::net::TcpStream;
use async_std::net::{SocketAddr, ToSocketAddrs};

pub struct Client {
    local_addr: SocketAddr,
    peer: Peer,
}

impl Client {
    pub async fn connect<A: ToSocketAddrs>(remote: A, tunnel_num: u32) -> Result<Self> {
        let peer_id = 0;
        let peer = Peer::client_side(peer_id);
        let mut local_addr = None;
        for _i in 0..tunnel_num {
            let stream = TcpStream::connect(&remote).await?;
            if local_addr.is_none() {
                local_addr = Some(stream.local_addr()?)
            }
            let tunnel = Tunnel::client_side(peer_id, stream).await?;

            let inbound_sender = peer.inbound_sender.clone();
            let outbound = peer.outbound.clone();
            spawn_and_log_err(tunnel.run(inbound_sender, outbound));
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
