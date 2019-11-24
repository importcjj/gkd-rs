use async_std::net::TcpStream;
use crate::Result;
use byteorder::{BigEndian, ReadBytesExt};
use async_std::prelude::*;
use std::io::Cursor;
use crate::packet::Packet;
use async_std::sync::{Sender, Receiver};
use futures::{FutureExt, StreamExt};
use std::marker::Unpin;
use async_std::io::{Write, Read};

pub struct Tunnel {
    pub peer_id: u32,
    stream: TcpStream,
}

impl Tunnel {
    pub async fn new_from_tcp_stream(mut stream: TcpStream) -> Result<Tunnel> {
        let mut buf = vec![0;4];
        stream.read_exact(&mut buf).await?;

        let peer_id = Cursor::new(buf).read_u32::<BigEndian>().unwrap();
        Ok(Tunnel {
            peer_id,
            stream,
        })
    }

    pub async fn run(mut self, 
        inbound_sender: Sender<Packet>,
        outbound_receiver: Receiver<Packet>,
    ) -> Result<()> {
        let (r, w) = &mut (&self.stream, &self.stream);
        
        futures::select!{
            r1 = inbound(r, inbound_sender).fuse() => r1?,
            r2 = outbound(w, outbound_receiver).fuse() => r2?,
        }

       
        Ok(())
    }
}

async fn inbound<R: Read + Unpin + ?Sized>(reader: &mut R, inbound_sender: Sender<Packet>) -> Result<()> {
    loop {
        let packet = Packet::from_reader(reader).await?;
        
    }
    Ok(())
}

async fn outbound<W: Write + Unpin + ?Sized>(writer: &mut W, outbound_receiver: Receiver<Packet>) -> Result<()> {
    Ok(())
}