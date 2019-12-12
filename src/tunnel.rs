use crate::packet::Packet;
use crate::peer::PeerGroup;
use crate::Result;
use async_std::io::{Read, Write};
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::sync::{Mutex, Weak};
use async_std::sync::{Receiver, Sender};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use futures::{FutureExt, StreamExt};
use log::debug;
use std::io::Cursor;
use std::marker::Unpin;
pub struct Tunnel {
    pub peer_id: u32,
    stream: TcpStream,
}

impl Tunnel {
    pub async fn client_side(peer_id: u32, mut stream: TcpStream) -> Result<Tunnel> {
        let mut buf = vec![];
        buf.write_u32::<BigEndian>(peer_id)?;

        stream.write_all(&buf).await?;
        Ok(Tunnel { peer_id, stream })
    }

    pub async fn server_side(mut stream: TcpStream) -> Result<Tunnel> {
        let mut buf = vec![0; 4];
        stream.read_exact(&mut buf).await?;

        let peer_id = Cursor::new(buf).read_u32::<BigEndian>().unwrap();
        Ok(Tunnel { peer_id, stream })
    }

    pub async fn run(
        self,
        inbound_sender: Sender<Packet>,
        outbound_receiver: Receiver<Packet>,
    ) -> Result<()> {
        let (r, w) = &mut (&self.stream, &self.stream);

        futures::select! {
            r1 = inbound(r, inbound_sender).fuse() => r1?,
            r2 = outbound(w, outbound_receiver).fuse() => r2?,
        }

        Ok(())
    }

    pub async fn run_with_peer(
        self,
        peers: Weak<Mutex<PeerGroup>>,
        inbound_sender: Sender<Packet>,
        outbound_receiver: Receiver<Packet>,
    ) -> Result<()> {
        let peer_id = self.peer_id;
        let r = self.run(inbound_sender, outbound_receiver).await;
        let peers = match peers.upgrade() {
            Some(peers) => peers,
            None => return Ok(()),
        };
        let mut peers = peers.lock().await;
        if peers.remove(&peer_id).is_some() {
            debug!("peer {} removed", peer_id);
        }
        r
    }
}

async fn inbound<R: Read + Unpin + ?Sized>(
    reader: &mut R,
    inbound_sender: Sender<Packet>,
) -> Result<()> {
    loop {
        let packet = Packet::from_reader(reader).await?;
        debug!("new packet {} - {}", packet.connection_id, packet.packet_id);
        inbound_sender.send(packet).await;
    }
}

async fn outbound<W: Write + Unpin + ?Sized>(
    writer: &mut W,
    mut outbound_receiver: Receiver<Packet>,
) -> Result<()> {
    while let Some(mut packet) = outbound_receiver.next().await {
        debug!("outbound {} - {}", packet.connection_id, packet.packet_id);
        let bytes = packet.pack();
        writer.write_all(bytes).await?;
    }
    Ok(())
}
