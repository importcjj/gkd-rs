use std::collections::HashMap;
use async_std::sync::{channel, Sender, Receiver};
use crate::packet::Packet;

pub type PeerGroup = HashMap<u32, Peer>;

pub struct Peer {
    peer_id: u32,
    inbound: Receiver<Packet>,
    pub inbound_sender: Sender<Packet>,
    pub outbound: Receiver<Packet>,
    outbound_sender: Sender<Packet>,
}

impl Peer {
    pub fn new(peer_id: u32) -> Self {
        let (inbound_sender, inbound) = channel(100);
        let (outbound_sender, outbound) = channel(100);
        Peer {
            peer_id,
            inbound,
            inbound_sender,
            outbound,
            outbound_sender,
        }
    }
}