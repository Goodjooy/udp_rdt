pub mod ack;
pub mod flags;
mod io;

use self::flags::{PackSplit, PacketType};

#[derive(Debug, Default)]
pub struct Packet {
    /// 差错校验码
    // verify: u16,
    packet_type: PacketType,
    packet_split: PackSplit,
    /// 当前packet 的编号
    /// 0 ~ 256
    identify_code: u8,
    /// body ,assume the body size <= 512
    body: Vec<u8>,
}

impl Packet {
    pub fn new(code: u8, body: Vec<u8>, ty: PacketType, split: PackSplit) -> Self {
        Self {
            identify_code: code,
            body,
            packet_type: ty,
            packet_split: split,
        }
    }

    pub fn new_data(code: u8, body: Vec<u8>) -> Self {
        Self::new(code, body, PacketType::Data, PackSplit::End)
    }

    pub fn get_body(self) -> Vec<u8> {
        self.body
    }
    pub fn get_id(&self) -> u8 {
        self.identify_code
    }
}
