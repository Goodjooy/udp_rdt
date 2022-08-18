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
    local_buf: Vec<u8>,
}

impl SelectResendReceiver {
    pub fn new(origin: SocketAddr) -> Self {
        Self {
            origin,
            buffer: FixedCycleBuffer::new(),
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
            }
            Err(_) => {
                // packet id mismatch , send last ack
            }
        }
        self.send_ack(buf, socket, packet_id).await?;
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

        Ok(vec)
    }

    pub async fn send_ack(&self, buf: &mut Vec<u8>, socket: &UdpSocket, ack: u8) -> io::Result<()> {
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
