
use async_std::net::TcpStream;


pub struct Connection {
    // stream: TcpStream,
// to_tunnel: Sender<Packet>,
// from_tunnel: Receiver<Packet>,
// ordered: Receiver<Packet>,
}

impl From<TcpStream> for Connection {
    fn from(_stream: TcpStream) -> Self {
        Self {}
    }
}
