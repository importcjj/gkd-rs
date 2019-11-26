use crate::packet::{Packet, PacketKind};
use crate::Result;
use async_std::io;
use async_std::io::{Read, Write};
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_std::pin::Pin;
use async_std::sync::{channel, Receiver, Sender};
use async_std::task::{Context, Poll};
use async_std::future::Future;
use bytes::BufMut;

pub struct Connection {
    connection_id: u32,

    send_id: u32,
    recv_id: u32,
    tunnel_sender: Sender<Packet>,
    tunnel_recv: Receiver<Packet>,
    ordered_sender: Sender<Packet>,
    ordered_recv: Receiver<Packet>,
}

impl Connection {
    pub(crate) async fn new<A: ToSocketAddrs>(
        dest: A,
        tunnel_sender: Sender<Packet>,
        tunnel_recv: Receiver<Packet>,
    ) -> Result<Self> {
        let (ordered_sender, ordered_recv) = channel(100);
        let mut conn = Self {
            connection_id: 0,
            send_id: 0,
            recv_id: 0,
            tunnel_sender,
            tunnel_recv,
            ordered_sender,
            ordered_recv,
        };

        conn.send_connect(dest).await?;
        Ok(conn)
    }

    async fn send_connect<A: ToSocketAddrs>(&mut self, dest: A) -> Result<()> {
        let addr = dest.to_socket_addrs().await?.next().unwrap();
        let address = format!("{}", addr);
        let connect = Packet::new_connect(self.send_id, self.connection_id, &address);
        self.send_id += 1;
        self.tunnel_sender.send(connect).await;
        Ok(())
    }

    async fn send_data(&mut self, buf: Vec<u8>) -> Result<()> {
        let data = Packet::new_data(self.send_id, self.connection_id, buf);
        self.send_id += 1;
        self.tunnel_sender.send(data).await;
        Ok(())
    }

    async fn send_disconnect(&mut self) -> Result<()> {
        let disconnect = Packet::new_disconnect(self.send_id, self.connection_id);
        self.send_id += 1;
        self.tunnel_sender.send(disconnect).await;
        Ok(())
    }

    async fn recv_data(&mut self) -> Result<Option<Packet>> {
        let packet = self.tunnel_recv.recv().await;
        Ok(packet)
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
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match Future::poll(Box::pin(*&mut self.recv_data()), cx) {
            Poll::Ready(Ok(Some(packet))) => match packet.kind {
                PacketKind::Connect => Poll::Ready(Ok(0)),
                PacketKind::Data => {
                    let data = packet.data.unwrap();
                    buf.put(&data);
                    Poll::Ready(Ok(data.len()))
                }
                PacketKind::Disconnect => Poll::Ready(Ok(0)),
            },
            Poll::Ready(Ok(None)) => Poll::Ready(Ok(0)),
            Poll::Pending => Poll::Pending,
            _ => panic!("poll read error"),
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
        match Pin::new(self.send_data(buf.to_vec())).poll(cx) {
            Poll::Ready(Ok(_)) => Poll::Ready(Ok(buf.len())),
            Poll::Ready(Ok(_)) => panic!("poll write error"),
            Poll::Pending => Poll::Pending,
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
