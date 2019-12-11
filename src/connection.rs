use crate::packet::{Packet, PacketKind};
use crate::spawn_and_log_err;
use crate::Result;
use async_std::future::Future;
use async_std::io;
use async_std::io::{Read, Write};
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_std::pin::Pin;
use async_std::prelude::*;
use async_std::sync::{channel, Mutex, Receiver, Sender};
use async_std::task;
use async_std::task::{Context, Poll};
use bytes::BufMut;
use futures::{select, FutureExt};
use log::debug;
use std::cmp;
use std::collections::HashMap;
use std::sync::Mutex as SyncMutex;
// use std::cell::RefCell;

pub struct Connection {
    pub connection_id: u32,
    send_id: Mutex<u32>,
    pub tunnel_sender: Sender<Packet>,
    ordered_recv: Receiver<Packet>,
    receiver: SyncMutex<Receiver<Packet>>,
}

impl Connection {
    fn new(
        connection_id: u32,
        tunnel_recv: Receiver<Packet>,
        tunnel_sender: Sender<Packet>,
    ) -> Self {
        let (ordered_sender, ordered_recv) = channel(100);
        task::spawn(order_packets(tunnel_recv, ordered_sender));
        Self {
            connection_id,
            send_id: Mutex::new(0),
            tunnel_sender,

            receiver: SyncMutex::new(ordered_recv.clone()),
            ordered_recv: ordered_recv,
        }
    }

    pub(crate) async fn client_side<A: ToSocketAddrs>(
        connection_id: u32,
        dest: A,
        tunnel_recv: Receiver<Packet>,
        tunnel_sender: Sender<Packet>,
    ) -> Result<Self> {
        let conn = Connection::new(connection_id, tunnel_recv, tunnel_sender);
        conn.send_connect(dest).await?;

        Ok(conn)
    }

    pub(crate) async fn serve(
        connection_id: u32,
        tunnel_recv: Receiver<Packet>,
        tunnel_sender: Sender<Packet>,
    ) -> Result<()> {
        let conn = Connection::new(connection_id, tunnel_recv, tunnel_sender);
        while let Some(packet) = conn.ordered_recv.recv().await {
            if packet.kind == PacketKind::Connect {
                let data = packet.data.as_ref().unwrap();
                let addr = String::from_utf8_lossy(data).to_string();
                debug!("connection to {}", addr);
                return conn.connect(addr).await;
            }
        }

        Ok(())
    }

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

        self.send_disconnect().await?;

        Ok(())
    }

    async fn send_connect<A: ToSocketAddrs>(&self, dest: A) -> Result<()> {
        let addr = dest.to_socket_addrs().await?.next().unwrap();
        let address = format!("{}", addr);
        let mut send_id = self.send_id.lock().await;
        let connect = Packet::new_connect(*send_id, self.connection_id, &address);
        *send_id += 1;
        self.tunnel_sender.send(connect).await;
        Ok(())
    }

    async fn send_data(&self, buf: Vec<u8>) -> Result<()> {
        debug!("send data {:?}", buf);
        let mut send_id = self.send_id.lock().await;
        let data = Packet::new_data(*send_id, self.connection_id, buf);
        *send_id += 1;
        self.tunnel_sender.send(data).await;
        Ok(())
    }

    async fn send_disconnect(&self) -> Result<()> {
        let mut send_id = self.send_id.lock().await;
        let disconnect = Packet::new_disconnect(*send_id, self.connection_id);
        *send_id += 1;
        self.tunnel_sender.send(disconnect).await;
        Ok(())
    }

    async fn recv_data(&self) -> Result<Option<Packet>> {
        debug!("read from ordered");
        let packet = self.ordered_recv.recv().await;
        debug!("inbound {:?}", packet);
        Ok(packet)
    }
}

async fn order_packets(inbound: Receiver<Packet>, ordered: Sender<Packet>) -> Result<()> {
    let mut recv_id = 0u32;
    let mut packets_caches = HashMap::<u32, Packet>::new();
    while let Some(packet) = inbound.recv().await {
        debug!("{} come in, {} expected", packet.packet_id, recv_id);
        if packet.packet_id == recv_id {
            ordered.send(packet).await;
            debug!("send ok");
            recv_id += 1;

            while let Some(packet) = packets_caches.remove(&recv_id) {
                debug!("{} in cache", packet.packet_id);
                ordered.send(packet).await;
                recv_id += 1;
            }
        } else {
            packets_caches.insert(packet.packet_id, packet);
        }
    }
    Ok(())
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

        let mut recv = self.receiver.lock().unwrap();
        let r = Pin::new(&mut *recv).poll_next(cx);

        debug!("read {:?}", r);

        match r {
            Poll::Ready(Some(packet)) => match packet.kind {
                PacketKind::Connect => Poll::Ready(Ok(0)),
                PacketKind::Data => {
                    let data = packet.data.unwrap();
                    debug!("poll read size {:?}", data.len());
                    let max = cmp::min(buf.len(), data.len());
                    debug!("target buf size: {}", max);
                    buf.put(&data[..max]);
                    Poll::Ready(Ok(max))
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
        debug!("write");
        let mut send_fut = Box::pin(self.send_data(buf.to_vec()));
        match send_fut.as_mut().poll(cx) {
            Poll::Ready(Ok(_)) => Poll::Ready(Ok(buf.len())),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => panic!("poll write error {:?}", e),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        // Pin::new(&mut &(*self).stream).poll_flush(cx)
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        // Pin::new(&mut &(*self).stream).poll_close(cx)
        Poll::Ready(Ok(()))
    }
}
