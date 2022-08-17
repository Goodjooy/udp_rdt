use super::{
    flags::{PackSplit, PacketType},
    Packet,
};

pub type Ack = Packet;

impl Ack {
    pub fn new_ack(code: u8) -> Self {
        Ack::new(code, vec![0xFF], PacketType::Ack, PackSplit::End)
    }

    /// 判断是否为ack 以及是否为对应code
    pub fn is_correct_ack(&self, code: u8) -> bool {
        self.identify_code == code && self.body == [0xFF]
    }

    pub fn get_ack_num(&self) -> u8 {
        self.identify_code
    }
}

#[cfg(test)]
mod test {
    use crate::packet::{
        flags::{PackSplit, PacketType},
        Packet,
    };

    use super::Ack;

    #[test]
    fn ack_gen_test() {
        let ack = Ack::new_ack(0);

        println!("{ack:?}")
    }
    #[test]
    fn ack_verify_test() {
        let ack = Ack::new_ack(0);

        // bad ack code
        assert!(!ack.is_correct_ack(1));

        // bad ack
        let ack = Packet::new(1, vec![0xFA], PacketType::Ack, PackSplit::End);

        assert!(!ack.is_correct_ack(1));

        // correct ack
        let ack = Ack::new_ack(0);
        assert!(ack.is_correct_ack(0));
    }
}
