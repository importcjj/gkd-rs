use crate::connection::Connection;
use crate::peer::Peer;
use crate::spawn_and_log_err;
use crate::tunnel::Tunnel;
use crate::Result;
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_std::task;

pub struct Client {
    peer: Peer,
}

impl Client {
    pub async fn new<A: ToSocketAddrs>(remote: A, tunnel_num: u32) -> Result<Self> {
        let peer_id = 0;
        let client = Client {
            peer: Peer::new(peer_id),
        };

        for _i in 0..tunnel_num {
            let stream = TcpStream::connect(&remote).await?;
            let tunnel = Tunnel::client_side(peer_id, stream).await?;

            let inbound_sender = client.peer.inbound_sender.clone();
            let outbound = client.peer.outbound.clone();
            spawn_and_log_err(tunnel.run(inbound_sender, outbound));
        }

        Ok(client)
    }

    pub async fn connect<A: ToSocketAddrs>(addrs: A) -> Result<Connection> {
        Ok(Connection {})
    }
}
