use crate::packet::{Packet, PacketKind};
use crate::Result;
use async_std::future::Future;
use async_std::io;
use async_std::io::{Read, Write};
use async_std::net::SocketAddr;
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_std::pin::Pin;
use async_std::prelude::*;
use async_std::sync::{channel, Receiver, Sender};
use async_std::task;
use async_std::task::{Context, Poll};
use bytes::BufMut;
use futures::{select, FutureExt};
use log::debug;
use std::cmp;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex as SyncMutex;

pub struct Connection {
    pub connection_id: u32,

    ordered_recv: Receiver<Packet>,
    recv_inner: SyncMutex<RecvInner>,

    sender_id: AtomicU32,
    sender: Sender<Packet>,
}

pub struct RecvInner {
    receiver: Receiver<Packet>,
    buf: bytes::BytesMut,
}

impl Connection {
    fn new(
        connection_id: u32,
        tunnel_recv: Receiver<Packet>,
        tunnel_sender: Sender<Packet>,
    ) -> Self {
        let (ordered_sender, ordered_recv) = channel(100);
        task::spawn(order_packets(tunnel_recv, ordered_sender));

        let recv_inner = RecvInner {
            receiver: ordered_recv.clone(),
            buf: bytes::BytesMut::new(),
        };

        Self {
            connection_id,

            ordered_recv: ordered_recv,
            recv_inner: SyncMutex::new(recv_inner),

            sender: tunnel_sender,
            sender_id: AtomicU32::new(0),
        }
    }

    pub(crate) async fn client_side(
        connection_id: u32,
        dest: SocketAddr,
        tunnel_recv: Receiver<Packet>,
        tunnel_sender: Sender<Packet>,
    ) -> Result<Self> {
        let conn = Connection::new(connection_id, tunnel_recv, tunnel_sender);
        conn.send_connect(dest).await?;

        Ok(conn)
    }

    pub(crate) async fn wait_connect_packet(
        connection_id: u32,
        tunnel_recv: Receiver<Packet>,
        tunnel_sender: Sender<Packet>,
        to_incomings: Sender<(Connection, SocketAddr)>,
    ) -> Result<()> {
        let conn = Connection::new(connection_id, tunnel_recv, tunnel_sender);
        while let Some(packet) = conn.ordered_recv.recv().await {
            if packet.kind == PacketKind::Connect {
                let data = packet.data.as_ref().unwrap();
                // FIXME
                let s = std::str::from_utf8(data).unwrap();
                let addr = s.to_socket_addrs().await?.next().unwrap();

                to_incomings.send((conn, addr.into())).await;
                debug!("connection to {}", addr);
                break;
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn connect<A: ToSocketAddrs>(self, dest: A) -> Result<()> {
        let target = TcpStream::connect(dest).await?;
        let (lr, lw) = &mut (&self, &self);
        let (tr, tw) = &mut (&target, &target);

        let copy_a = io::copy(lr, tw);
        let copy_b = io::copy(tr, lw);

        let _ = select! {
            r1 = copy_a.fuse() => r1?,
            r2 = copy_b.fuse() => r2?
        };

        Ok(())
    }

    async fn send_connect(&self, dest: SocketAddr) -> Result<()> {
        let address = format!("{}", dest);
        let send_id = self.sender_id.fetch_add(1, Ordering::Relaxed);
        let connect = Packet::new_connect(send_id, self.connection_id, &address);

        self.sender.send(connect).await;
        Ok(())
    }

    async fn send_data(&self, buf: Vec<u8>) -> Result<()> {
        let send_id = self.sender_id.fetch_add(1, Ordering::Relaxed);
        let data = Packet::new_data(send_id, self.connection_id, buf);
        self.sender.send(data).await;
        Ok(())
    }
}

async fn order_packets(inbound: Receiver<Packet>, ordered: Sender<Packet>) -> Result<()> {
    let mut recv_id = 0u32;
    let mut packets_caches = HashMap::<u32, Packet>::new();
    while let Some(packet) = inbound.recv().await {
        debug!("{} come in, {} expected", packet.packet_id, recv_id);
        if packet.packet_id == recv_id {
            ordered.send(packet).await;
            recv_id += 1;

            while let Some(packet) = packets_caches.remove(&recv_id) {
                debug!("{} - {} in cache", packet.connection_id, packet.packet_id);
                ordered.send(packet).await;
                recv_id += 1;
            }
        } else {
            packets_caches.insert(packet.packet_id, packet);
        }
    }
    Ok(())
}

impl Drop for Connection {
    fn drop(&mut self) {
        let send_id = self.sender_id.fetch_add(1, Ordering::Relaxed);
        let disconnect = Packet::new_disconnect(send_id, self.connection_id);
        let sender = self.sender.clone();
        task::spawn(async move {
            sender.send(disconnect).await;
        });
    }
}

impl Read for Connection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_read(cx, buf)
    }
}

impl Read for &Connection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        mut buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut inner = self.recv_inner.lock().unwrap();

        if !inner.buf.is_empty() {
            let expected = buf.len();
            let remain = inner.buf.len();
            let split_at = cmp::min(expected, remain);
            let data = inner.buf.split_to(split_at);
            buf.put(data);
            return Poll::Ready(Ok(split_at));
        }

        let r = Pin::new(&mut inner.receiver).poll_next(cx);

        // debug!("read {:?}", r);

        match r {
            Poll::Ready(Some(packet)) => match packet.kind {
                PacketKind::Connect => Poll::Ready(Ok(0)),
                PacketKind::Data => {
                    let data = packet.data.unwrap();
                    debug!("poll read size {:?}", data.len());
                    let min = cmp::min(buf.len(), data.len());
                    inner.buf.extend_from_slice(&data);

                    let data = inner.buf.split_to(min);
                    buf.put(data);
                    return Poll::Ready(Ok(min));
                }
                PacketKind::Disconnect => Poll::Ready(Ok(0)),
            },
            Poll::Ready(None) => Poll::Ready(Ok(0)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Write for Connection {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_close(cx)
    }
}

impl io::Write for &Connection {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        debug!(
            "connection {} poll write {:?}",
            self.connection_id,
            buf.len()
        );
        let mut send_fut = Box::pin(self.send_data(buf.to_vec()));
        let r = send_fut.as_mut().poll(cx);
        debug!("connection {} write resp {:?}", self.connection_id, r);
        match r {
            Poll::Ready(Ok(_)) => Poll::Ready(Ok(buf.len())),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => panic!("poll write error {:?}", e),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        debug!("connection {} poll flush", self.connection_id);
        // Pin::new(&mut &(*self).stream).poll_flush(cx)
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        debug!("connection {} poll close", self.connection_id);
        // Pin::new(&mut &(*self).stream).poll_close(cx)
        Poll::Ready(Ok(()))
    }
}
