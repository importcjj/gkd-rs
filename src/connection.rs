use crate::packet::Packet;
use async_std::net::TcpStream;
use async_std::sync::{channel, Receiver, Sender};

pub struct Connection {
    // stream: TcpStream,
// to_tunnel: Sender<Packet>,
// from_tunnel: Receiver<Packet>,
// ordered: Receiver<Packet>,
}

impl From<TcpStream> for Connection {
    fn from(stream: TcpStream) -> Self {
        Self {}
    }
}
