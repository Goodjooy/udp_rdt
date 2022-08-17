use std::{io, net::SocketAddr};

use crate::{
    fake_udp::UdpSocket,
    packet::{ack::Ack, Packet},
};

use super::GbnError;

pub struct GoBackNReceiver {
    origin: SocketAddr,
    last_ack: Ack,
    pkg_id: u8,
}

impl GoBackNReceiver {
    pub fn new(origin: SocketAddr) -> Self {
        Self {
            origin,
            last_ack: Ack::new_ack(u8::MAX),
            pkg_id: 0,
        }
    }

    pub async fn receive(
        &mut self,
        buf: &mut Vec<u8>,
        packet: Packet,
        socket: UdpSocket,
    ) -> Result<Vec<u8>, GbnError> {
        let resp = if packet.get_id() == self.pkg_id {
            self.last_ack = Ack::new_ack(self.pkg_id);
            self.pkg_id = self.pkg_id.wrapping_add(1);
            Ok(packet.get_body())
        } else {
            Err(GbnError::PacketIdMisMatch)
        };

        self.send_ack(buf, socket).await?;

        resp
    }

    pub async fn send_ack(&self, buf: &mut Vec<u8>, socket: UdpSocket) -> io::Result<()> {
        buf.clear();
        let size = self.last_ack.write(buf)?;
        let send_body = &buf[0..size];

        socket.send_to(send_body, self.origin).await?;

        Ok(())
    }
}
