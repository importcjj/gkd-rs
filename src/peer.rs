use crate::connection::Connection;
use crate::packet::Packet;
use crate::spawn_and_log_err;
use crate::Result;
use async_std::net::ToSocketAddrs;
use async_std::sync::{channel, Mutex, Receiver, Sender};
use log::debug;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

static CONNECTION_ID: AtomicU32 = AtomicU32::new(0);

pub type PeerGroup = HashMap<u32, Peer>;

pub struct Peer {
    peer_id: u32,
    pub inbound: Receiver<Packet>,
    pub inbound_sender: Sender<Packet>,
    pub outbound: Receiver<Packet>,
    pub outbound_sender: Sender<Packet>,
    pub connection_recvs: Arc<Mutex<HashMap<u32, Sender<Packet>>>>,
}

impl Peer {
    pub fn new(peer_id: u32) -> Self {
        let (inbound_sender, inbound) = channel(1024);
        let (outbound_sender, outbound) = channel(1024);
        Peer {
            peer_id,
            inbound,
            inbound_sender,
            outbound,
            outbound_sender,
            connection_recvs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn client_side(peer_id: u32) -> Self {
        let peer = Peer::new(peer_id);
        let inbound = peer.inbound.clone();
        let dispatch = peer.connection_recvs.clone();
        spawn_and_log_err(peer_loop_client_side(inbound, dispatch));
        peer
    }

    pub fn server_side(peer_id: u32) -> Self {
        let peer = Peer::new(peer_id);
        let inbound = peer.inbound.clone();
        let outbound = peer.outbound_sender.clone();
        spawn_and_log_err(peer_loop_server_side(inbound, outbound));
        peer
    }

    pub async fn new_client_side_connection<A: ToSocketAddrs>(
        &self,
        dest: A,
    ) -> Result<Connection> {
        let (send_to_conn, conn_recv) = channel(1024);
        let id = CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
        debug!("make new connection {:?}", id);

        let conn =
            Connection::client_side(id, dest, conn_recv, self.outbound_sender.clone()).await?;
        let mut recvs = self.connection_recvs.lock().await;

        recvs.insert(id, send_to_conn);
        Ok(conn)
    }
}

async fn peer_loop_client_side(
    inbound: Receiver<Packet>,
    dispath: Arc<Mutex<HashMap<u32, Sender<Packet>>>>,
) -> Result<()> {
    while let Some(packet) = inbound.recv().await {
        debug!(
            "client recv new {} - {}",
            packet.connection_id, packet.packet_id
        );
        debug!("try to lock the dispatch");
        let dispatch_guard = dispath.lock().await;
        debug!("dispatch locked");

        match dispatch_guard.get(&packet.connection_id) {
            Some(ref sender) => {
                debug!("send to channel");
                // FIXME: should not drop the packet
                if !sender.is_full() {
                    sender.send(packet).await;
                    debug!("sended to channel");
                } else {
                    debug!("packet dropped");
                }
            }
            None => debug!("nothing to do"),
        }
        drop(dispatch_guard);
    }

    Ok(())
}

async fn peer_loop_server_side(
    inbound: Receiver<Packet>,
    outbound_sender: Sender<Packet>,
) -> Result<()> {
    let mut dispatch: HashMap<u32, Sender<Packet>> = HashMap::new();

    while let Some(packet) = inbound.recv().await {
        debug!(
            "server recv new {} - {}",
            packet.connection_id, packet.packet_id
        );
        match dispatch.get(&packet.connection_id) {
            Some(ref sender) => {
                sender.send(packet).await;
            }
            None => {
                debug!("make new connection");
                let (send_to_conn, conn_recv) = channel(1024);
                spawn_and_log_err(Connection::serve(
                    packet.connection_id,
                    conn_recv,
                    outbound_sender.clone(),
                ));

                let connection_id = packet.connection_id;
                send_to_conn.send(packet).await;
                dispatch.insert(connection_id, send_to_conn);
            }
        }
    }

    debug!("peer_loop_server_side finished");
    Ok(())
}
