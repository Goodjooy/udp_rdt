use std::{io, net::SocketAddr};

use crate::{
    fake_udp::UdpSocket,
    fixed_cycle_buf::FixedCycleBuffer,
    packet::{ack::Ack, flags::PackSplit, Packet},
};

use super::{SrError, MAX_WINDOWS_SIZE};

pub struct SelectResendReceiver {
    origin: SocketAddr,
    buffer: FixedCycleBuffer<MAX_WINDOWS_SIZE, RecvWrap>,
    last_ack: [u8; MAX_WINDOWS_SIZE as usize],
    local_buf: Vec<u8>,
}

impl SelectResendReceiver {
    pub fn new(origin: SocketAddr) -> Self {
        let ack = {
            let mut ack = [0u8; MAX_WINDOWS_SIZE as usize];
            (0..MAX_WINDOWS_SIZE)
                .map(|v| v << 4)
                .enumerate()
                .for_each(|(idx, last_ack)| {
                    ack[idx] = last_ack;
                });
            ack
        };

        Self {
            origin,
            buffer: FixedCycleBuffer::new(),
            last_ack: ack,
            local_buf: Vec::new(),
        }
    }

    pub async fn receive(
        &mut self,
        buf: &mut Vec<u8>,
        packet: Packet,
        socket: &UdpSocket,
    ) -> Result<Vec<Vec<u8>>, SrError> {
        let packet_id = packet.get_id();
        let offset = self.buffer.calculate_offset(packet_id);
        // the packet id is in the windows
        match self.buffer.insert(
            packet_id,
            RecvWrap {
                split: packet.packet_split(),
                packet: packet.get_body(),
            },
        ) {
            Ok(_) => {
                // recv packet ok, update ack
                self.last_ack[offset as usize] = packet_id;
            }
            Err(_) => {
                // packet id mismatch , send last ack
            }
        }
        // slide windows
        let mut vec = Vec::new();
        self.buffer.slide_windows().into_iter().for_each(
            |RecvWrap { split, packet }| match split {
                PackSplit::End => vec.push({
                    let mut v = std::mem::take(&mut self.local_buf);
                    v.extend(packet);
                    v
                }),
                PackSplit::Follow => self.local_buf.extend(packet),
            },
        );

        self.send_ack(buf, socket, offset).await?;

        Ok(vec)
    }

    pub async fn send_ack(
        &self,
        buf: &mut Vec<u8>,
        socket: &UdpSocket,
        offset: u8,
    ) -> io::Result<()> {
        let ack = self.last_ack[offset as usize];
        let ack = Ack::new_ack(ack);

        // write
        buf.clear();
        let size = ack.write(buf)?;
        let ack_packet = &buf[0..size];

        // send
        socket.send_to(ack_packet, self.origin).await?;
        println!(
            "Sending Ack [{}] to Socket {}",
            ack.get_ack_num(),
            self.origin
        );

        Ok(())
    }
}

struct RecvWrap {
    split: PackSplit,
    packet: Vec<u8>,
}
