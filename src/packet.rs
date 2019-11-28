use crate::Result;
use async_std::io::Read;
use async_std::prelude::*;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;
use std::marker::Unpin;

const PACKET_HEAD_SIZE: usize = 13;

#[derive(Debug)]
pub struct Packet {
    pub kind: PacketKind,
    pub connection_id: u32,
    pub packet_id: u32,
    pub data_length: u32,
    pub data: Option<Vec<u8>>,
    cache: Option<Vec<u8>>,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PacketKind {
    Connect = 1,
    Disconnect = 2,
    Data = 3,
}

impl Packet {
    pub async fn from_reader<R>(reader: &mut R) -> Result<Packet>
    where
        R: Read + Unpin + ?Sized,
    {
        let mut header = vec![0; PACKET_HEAD_SIZE];
        reader.read_exact(&mut header).await?;

        let pakcet_kind = match header[0] {
            1 => PacketKind::Connect,
            2 => PacketKind::Disconnect,
            3 => PacketKind::Data,
            _ => unreachable!(),
        };

        let data_length = Cursor::new(&header[9..13]).read_u32::<BigEndian>().unwrap();
        let mut data = vec![0; data_length as usize];
        reader.read_exact(&mut data).await?;

        Ok(Packet {
            kind: pakcet_kind,
            connection_id: Cursor::new(&header[1..5]).read_u32::<BigEndian>().unwrap(),
            packet_id: Cursor::new(&header[5..9]).read_u32::<BigEndian>().unwrap(),
            data_length,
            data: Some(data),
            cache: None,
        })
    }
}

impl Packet {
    pub fn pack(&mut self) -> &[u8] {
        if self.cache.is_some() {
            return self.cache.as_ref().unwrap();
        }

        let mut packed = vec![];
        packed.push(self.kind as u8);
        packed.write_u32::<BigEndian>(self.connection_id);
        packed.write_u32::<BigEndian>(self.packet_id);
        packed.write_u32::<BigEndian>(self.data_length);

        if let Some(ref data) = self.data.as_ref() {
            packed.extend_from_slice(data);
        }

        self.cache = Some(packed);
        self.cache.as_ref().unwrap()
    }

    pub fn new_connect(packet_id: u32, connection_id: u32, address: &str) -> Packet {
        let data = address.as_bytes();
        Self {
            kind: PacketKind::Connect,
            connection_id,
            packet_id,
            data_length: data.len() as u32,
            data: Some(data.to_vec()),
            cache: None,
        }
    }

    pub fn new_data(packet_id: u32, connection_id: u32, data: Vec<u8>) -> Packet {
        Self {
            kind: PacketKind::Data,
            connection_id,
            packet_id,
            data_length: data.len() as u32,
            data: Some(data),
            cache: None,
        }
    }

    pub fn new_disconnect(packet_id: u32, connection_id: u32) -> Packet {
        Self {
            kind: PacketKind::Disconnect,
            connection_id,
            packet_id,
            data_length: 0,
            data: None,
            cache: None,
        }
    }
}
