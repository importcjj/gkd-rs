use crate::packet::{Packet, PacketKind};
use crate::Result;
use async_std::task;
use async_std::future::Future;
use async_std::io;
use async_std::prelude::*;
use async_std::io::{Write, Read};
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_std::pin::Pin;
use async_std::sync::{channel, Mutex, Receiver, Sender};
use async_std::task::{Context, Poll};
use bytes::BufMut;
use std::collections::HashMap;
use log::debug;

pub struct Connection {
    connection_id: u32,
    send_id: Mutex<u32>,
    tunnel_sender: Sender<Packet>,
    ordered_recv: Receiver<Packet>,
}

impl Connection {
    pub(crate) async fn new<A: ToSocketAddrs>(
        dest: A,
        tunnel_recv: Receiver<Packet>,
        tunnel_sender: Sender<Packet>,
    ) -> Result<Self> {
        let (ordered_sender, ordered_recv) = channel(100);
        let mut conn = Self {
            connection_id: 0,
            send_id: Mutex::new(0),
            tunnel_sender,
            ordered_recv,
        };

        task::spawn(order_packets(tunnel_recv, ordered_sender));
        conn.send_connect(dest).await?;

        Ok(conn)
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
        let packet = self.ordered_recv.recv().await;
        debug!("inbound {:?}", packet);
        Ok(packet)
    }

    
}

async fn order_packets(inbound: Receiver<Packet>, ordered: Sender<Packet>) -> Result<()> {
    let mut recv_id = 0u32;
    let mut packets_caches = HashMap::<u32, Packet>::new();
    while let Some(packet) = inbound.recv().await {
        println!("!!!!!order");
        if packet.packet_id == recv_id {
            ordered.send(packet).await;
            recv_id += 1;

            while let Some(packet) = packets_caches.remove(&recv_id) {
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
        let mut recv_fut = Box::pin(self.recv_data());
        match recv_fut.as_mut().poll(cx) {
            Poll::Ready(Ok(Some(packet))) => match packet.kind {
                PacketKind::Connect => Poll::Ready(Ok(0)),
                PacketKind::Data => {
                    let data = packet.data.unwrap();
                    buf.put(&data[..]);
                    Poll::Ready(Ok(data.len()))
                }
                PacketKind::Disconnect => Poll::Ready(Ok(0)),
            },
            Poll::Ready(Ok(None)) => Poll::Ready(Ok(0)),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => panic!("poll read error {:?}", e),
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
