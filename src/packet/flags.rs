use std::{ops::{BitOr, BitOrAssign}, io::{Read, self}};

use byteorder::ReadBytesExt;
#[derive(PartialEq, Eq)]
pub struct PacketFlag(u8);

/// Packet split: End
const END_PACKET: PacketFlag = PacketFlag::new(0o001); //001
/// Packet split: part(not end)
const FOLLOW_PACKET: PacketFlag = PacketFlag::new(0o002); //010
#[derive(Debug, Clone, Copy)]
pub enum PackSplit {
    End,
    Follow,
}

/// Body type
const U64_SIZE: PacketFlag = PacketFlag::new(0o010); //001
const U32_SIZE: PacketFlag = PacketFlag::new(0o020); //
const U16_SIZE: PacketFlag = PacketFlag::new(0o030);
const U8_SIZE: PacketFlag = PacketFlag::new(0o040);
const SINGLE: PacketFlag = PacketFlag::new(0o050);
const EMPTY: PacketFlag = PacketFlag::new(0o060);
#[derive(Debug, Clone, Copy)]
pub enum BodySize {
    U64,
    U32,
    U16,
    U8,
    Single,
    Empty,
}

/// packet type

const DATA: PacketFlag = PacketFlag::new(0b01_000_000);
const ACK: PacketFlag = PacketFlag::new(0b10_000_000);

#[derive(Debug, Clone, Copy)]
pub enum PacketType {
    Data,
    Ack,
}

impl PacketFlag {
    const fn new(flag: u8) -> Self {
        Self(flag)
    }

    pub fn from_reader<R:Read>(reader:&mut R)->io::Result<Self>{
        Ok(Self::new(reader.read_u8()?))
    }

    pub fn get_flag(&self) -> u8 {
        self.0
    }
}

impl BitOr for PacketFlag {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::new(self.0 | rhs.0)
    }
}

impl BitOrAssign for PacketFlag {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl PacketFlag {
    pub fn contains(&self, rhs: &Self) -> bool {
        self.0 & rhs.0 > 0
    }

    pub fn from_packet_info(size: usize, split: PackSplit, ty: PacketType) -> Self {
        let size = if size == 0 {
            EMPTY
        } else if size == 1 {
            SINGLE
        } else if size <= u8::MAX as usize {
            U8_SIZE
        } else if size <= u16::MAX as usize {
            U16_SIZE
        } else if size <= u32::MAX as usize {
            U32_SIZE
        } else {
            U64_SIZE
        };

        let split = match split {
            PackSplit::End => END_PACKET,
            PackSplit::Follow => FOLLOW_PACKET,
        };

        let ty = match ty {
            PacketType::Data => DATA,
            PacketType::Ack => ACK,
        };

        size | split | ty
    }

    pub fn get_pack_size(&self) -> Option<BodySize> {
        match Self(self.0 & 0o070) {
            U64_SIZE => Some(BodySize::U64),
            U32_SIZE => Some(BodySize::U32),
            U16_SIZE => Some(BodySize::U16),
            U8_SIZE => Some(BodySize::U8),
            SINGLE => Some(BodySize::Single),
            EMPTY => Some(BodySize::Empty),
            _ => None,
        }
    }

    pub fn get_pack_split(&self) -> Option<PackSplit> {
        match Self(self.0 & 0b00_000_111) {
            END_PACKET => Some(PackSplit::End),
            FOLLOW_PACKET => Some(PackSplit::Follow),
            _ => None,
        }
    }

    pub fn get_pack_type(&self) -> Option<PacketType> {
        match Self(self.0 & 0b11_000_000) {
            DATA => Some(PacketType::Data),
            ACK => Some(PacketType::Ack),
            _ => None,
        }
    }
}
