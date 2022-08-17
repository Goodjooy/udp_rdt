use std::{io::Write, net::SocketAddr, sync::Arc, time::Duration};

use tokio::sync::mpsc;

use crate::{
    fake_udp::UdpSocket,
    packet::Packet,
    slide_windows::{NeedTimeoutWork, StatePacket, Timer, MAX_BUFF_SIZE, TIMEOUT_MS},
};

use super::GbnError;

const MAX_WINDOWS: usize = 255;

pub struct GoBackNSender {
    target: SocketAddr,
    /// using 8bit for packet id
    /// max windows is 255
    /// when packet id = 0 NAK => ACK 255
    /// but if the packet id = 255 is waiting too, thus
    /// course confuse
    ///
    /// max windows size is 255 = 2 ^ 8 - 1
    /// 0~254
    buffer: Vec<StatePacket>,
    /// 0 ~ 254
    head: u8,
    /// 0 ~ 254
    end: u8,
    /// buffer size
    size: u8,
    /// timer
    timer: Option<Timer>,
}

impl GoBackNSender {
    pub fn new(target: SocketAddr) -> Self {
        Self {
            target,
            buffer: (0..MAX_WINDOWS).map(|_| StatePacket::default()).collect(),
            head: 0,
            end: 0,
            size: 0,
            timer: None,
        }
    }

    pub async fn send(
        &mut self,
        buf: &mut Vec<u8>,
        body: Vec<u8>,
        socket: &UdpSocket,
        timeout_send: mpsc::Sender<()>,
    ) -> Result<(), super::GbnError> {
        if (self.size) as usize > MAX_WINDOWS {
            Err(GbnError::BufferFilled)?
        }

        // 封装包
        let packet = Packet::new_data(self.head, body);
        let size = packet.write(buf)?;
        let send_packet = &buf[0..size];

        // set packet to buffer
        self.buffer[self.head as usize] = StatePacket::new_waiting(packet);
        self.head = self.head.wrapping_add(1);
        self.size += 1;

        // send packet
        let len = socket.send_to(send_packet, self.target).await?;
        println!("Send body size {len}");

        // start timer

        // if this packet is the first, start a timer
        if self.size == 1 {
            let (timer, timeout) = Timer::start(Duration::from_millis(TIMEOUT_MS));
            // stop last timer
            self.timer.replace(timer).map(|t| t.stop());

            // if time out send again
            let task = async move {
                match timeout.waiting().await {
                    NeedTimeoutWork::Need => {
                        timeout_send.send(()).await.ok();
                    }
                    NeedTimeoutWork::None => (),
                }
            };
            tokio::task::spawn(task);
            tokio::task::yield_now().await;
        }

        Ok(())
    }

    /// 在go back n 中， ack 是累计校验
    /// 即在缓冲区里面 packet id <= ack 的均为被收到且通过校验
    /// 接收端的缓冲区只有1
    pub async fn recv_ack(&mut self, ack_num: u8, timeout_send: mpsc::Sender<()>) {
        // ----end---ack_num-----------head
        if self.end <= ack_num {
            self.end = ack_num.wrapping_add(1);
            //start a new timer
            let (timer, timeout) = Timer::start(Duration::from_millis(TIMEOUT_MS));
            // stop old timer
            self.timer.replace(timer).map(|v| v.stop());

            tokio::task::spawn(async move {
                match timeout.waiting().await {
                    NeedTimeoutWork::Need => {
                        timeout_send.send(()).await.ok();
                    }
                    NeedTimeoutWork::None => (),
                }
            });
            tokio::task::yield_now().await;
        } else {
            println!(
                "ACK num {ack_num} smaller then end {} , waiting for time out re send all",
                self.end
            )
        }
    }

    /// resend all packet in buffer that not recv ACK
    pub async fn resend_all(
        &mut self,
        socket: &UdpSocket,
        timeout_send: mpsc::Sender<()>,
    ) -> Result<(), GbnError> {
        // stop old timer
        self.timer.take().map(|v| v.stop());

        // resend all data
        let mut buf = <Vec<u8>>::with_capacity(MAX_BUFF_SIZE);

        let mut idx = self.end;

        while idx == self.head {
            // the packet is always exist
            let packet = self.buffer.get_mut(idx as usize).unwrap();
            buf.clear();
            let size = packet.pkg.write(&mut buf)?;
            let send_packet = &buf[0..size];

            let len = socket.send_to(send_packet, self.target).await?;
            println!("Resend Packet {} size: [{}]", idx, len);

            // update idx
            idx = idx.wrapping_add(1);
        }

        // create new timer
        let (timer, timeout) = Timer::start(Duration::from_millis(TIMEOUT_MS));
        self.timer = Some(timer);

        tokio::task::spawn(async move {
            match timeout.waiting().await {
                NeedTimeoutWork::Need => {
                    timeout_send.send(()).await.ok();
                }
                NeedTimeoutWork::None => (),
            }
        });
        tokio::task::yield_now().await;

        Ok(())
    }
}
