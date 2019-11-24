use async_std::io::Read;
use std::marker::Unpin;
use crate::Result;

const PACKET_HEAD_SIZE: usize = 13;

pub struct Packet {
    kind: PacketKind,
    connection_id: u32, 
    packet_id: u32,
    data_length: u32,
    data: Vec<u8>,
    cache: Vec<u8>,
}

#[repr(u8)]
pub enum PacketKind {
    Connect = 1,
    Disconnect = 2,
    Data = 3,
}

impl Packet {
    pub async fn from_reader<R>(reader: &mut R) -> Result<Packet>
    where
        R: Read + Unpin + ?Sized
    {
        unimplemented!()
    }
}