use crate::connection::Connection;
use crate::peer::Peer;
use crate::spawn_and_log_err;
use crate::tunnel::Tunnel;
use crate::Result;
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;

pub struct Client {
    peer: Peer,
}

impl Client {
    pub async fn new<A: ToSocketAddrs>(remote: A, tunnel_num: u32) -> Result<Self> {
        let peer_id = 0;
        let peer = Peer::client_side(peer_id);
        for _i in 0..tunnel_num {
            let stream = TcpStream::connect(&remote).await?;
            let tunnel = Tunnel::client_side(peer_id, stream).await?;

            let inbound_sender = peer.inbound_sender.clone();
            let outbound = peer.outbound.clone();
            spawn_and_log_err(tunnel.run(inbound_sender, outbound));
        }

        Ok(Client { peer })
    }

    pub async fn connect<A: ToSocketAddrs>(&self, addr: A) -> Result<Connection> {
        self.peer.new_client_side_connection(addr).await
    }
}
