use std::{net::SocketAddr, time::Duration};

use tokio::sync::mpsc;

use crate::{
    cycle_buffer::CycleBuffer,
    fake_udp::UdpSocket,
    packet::Packet,
    slide_windows::{Timer, TIMEOUT_MS},
};

use super::{SrError, MAX_WINDOWS_SIZE};



pub struct SelectResendSender {
    target: SocketAddr,
    buffer: CycleBuffer<MAX_WINDOWS_SIZE, (Timer, Packet)>,
}

impl SelectResendSender {
    pub fn new(target: SocketAddr) -> Self {
        Self {
            target,
            buffer: CycleBuffer::new(),
        }
    }

    pub async fn send(
        &mut self,
        buf: &mut Vec<u8>,
        body: Vec<u8>,
        socket: &UdpSocket,
        timeout_send: mpsc::Sender<u8>,
    ) -> Result<(), SrError> {
        let this_id = self.buffer.top();
        // 封装包
        let packet = Packet::new_data(this_id, body);
        buf.clear();
        let size = packet.write(buf)?;
        let send_packet = &buf[0..size];

        let (timer, starter, timeout) = Timer::later_start(Duration::from_millis(TIMEOUT_MS));
        timeout.need_resend_do(async move {
            eprintln!("waiting timeout, resend");
            timeout_send.send(this_id).await.ok();
        });
        // packet 加入缓冲区
        self.buffer.push((timer, packet))?;
        println!("Add Packet to Buffer now size : [{}]", self.buffer.len());

        // send packet
        socket.send_to(send_packet, self.target).await?;
        println!("Send Packet done [{}]", this_id);

        // start timer
        starter.start();

        Ok(())
    }

    pub async fn recv_ack(&mut self, ack: u8) {
        if let Some((timer, _)) = self.buffer.get(ack) {
            // target ack is on waiting, recv it ack ,can stop timer;
            timer.stop();
        }

        self.buffer.buffer_down(ack);
        self.buffer.slide_buff();
    }

    pub async fn select_resend(
        &mut self,
        packet_id: u8,
        buf: &mut Vec<u8>,
        socket: &UdpSocket,
        timeout_send: mpsc::Sender<u8>,
    ) -> Result<(), SrError> {
        if let Some((src_timer, packet)) = self.buffer.get_mut(packet_id) {
            src_timer.stop();
            let (timer, starter, timeout) = Timer::later_start(Duration::from_millis(TIMEOUT_MS));
            *src_timer = timer;
            timeout.need_resend_do(async move {
                eprintln!("waiting timeout , resend");
                timeout_send.send(packet_id).await.ok();
            });

            //write packet
            buf.clear();
            let size = packet.write(buf)?;
            let send_packet = &buf[0..size];

            // send packet
            socket.send_to(send_packet, self.target).await?;
            println!("Resend Packet [{}] done", packet.get_id());
            // start timer
            starter.start();
        }
        Ok(())
    }
}
