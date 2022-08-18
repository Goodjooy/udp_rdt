use std::{net::SocketAddr, time::Duration};

use tokio::sync::mpsc;

use crate::{
    cycle_buffer::CycleBuffer,
    fake_udp::UdpSocket,
    packet::Packet,
    slide_windows::{NeedTimeoutWork, Timer, TIMEOUT_MS},
};

use super::GbnError;

const MAX_WINDOWS: u8 = 255;

pub struct GoBackNSender {
    target: SocketAddr,
    /// using 8bit for packet id
    ///
    /// max windows is 255
    ///
    /// when packet id = 0 NAK => ACK 255
    ///
    /// but if the packet id = 255 is waiting too, thus
    ///
    /// course confuse
    ///
    /// max windows size is 255 = 2 ^ 8 - 1
    /// 0~254
    buffer: CycleBuffer<MAX_WINDOWS, Packet>,
    /// timer
    timer: Option<Timer>,
}

impl GoBackNSender {
    pub fn new(target: SocketAddr) -> Self {
        Self {
            target,
            buffer: CycleBuffer::new(),
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
        // 封装包
        let packet = Packet::new_data(self.buffer.top(), body);
        buf.clear();
        let size = packet.write(buf)?;
        let send_packet = &buf[0..size];

        // set packet to buffer
        self.buffer.push(packet)?;
        println!("updated size: {}", self.buffer.len());

        // send packet
        let len = socket.send_to(send_packet, self.target).await?;
        println!(
            "Send Packet {} size {len}",
            self.buffer.top().wrapping_sub(1)
        );

        // start timer

        // if this packet is the first, start a timer
        if self.buffer.len() == 1 {
            let (timer, timeout) = Timer::start(Duration::from_millis(TIMEOUT_MS));
            // stop last timer
            self.timer.replace(timer).map(|t| t.stop());

            // if time out send again
            tokio::task::spawn(async move {
                match timeout.waiting().await {
                    NeedTimeoutWork::Need => {
                        eprintln!("waiting timeout , resend");
                        timeout_send.send(()).await.ok();
                    }
                    NeedTimeoutWork::None => (),
                }
            });
            tokio::task::yield_now().await;
        }

        Ok(())
    }

    /// 在go back n 中， ack 是累计校验
    /// 即在缓冲区里面 packet id <= ack 的均为被收到且通过校验
    /// 接收端的缓冲区只有1
    pub async fn recv_ack(&mut self, ack_num: u8, timeout_send: mpsc::Sender<()>) {
        // ----end---ack_num-----------head
        // ack 在end 紧接着的上一个位置，那么就是NAK
        match self.buffer.set_button(ack_num) {
            Ok(_) => {
                println!("updated size: {}", self.buffer.len());
                println!("ACK PASS");

                if self.buffer.len() > 0 {
                    //start a new timer
                    let (timer, timeout) = Timer::start(Duration::from_millis(TIMEOUT_MS));
                    // stop old timer
                    self.timer.replace(timer).map(|v| v.stop());

                    tokio::task::spawn(async move {
                        match timeout.waiting().await {
                            NeedTimeoutWork::Need => {
                                eprintln!("waiting timeout, resend");
                                timeout_send.send(()).await.ok();
                            }
                            NeedTimeoutWork::None => (),
                        }
                    });
                    tokio::task::yield_now().await;
                } else {
                    self.timer.take().map(|v| v.stop());
                }
            }
            Err(_) => {
                println!(
                    "ACK num {ack_num} smaller then end {} , waiting for time out re send all",
                    self.buffer.button()
                )
            }
        }
    }

    /// resend all packet in buffer that not recv ACK
    pub async fn resend_all(
        &mut self,
        buf: &mut Vec<u8>,
        socket: &UdpSocket,
        timeout_send: mpsc::Sender<()>,
    ) -> Result<(), GbnError> {
        // stop old timer
        self.timer.take().map(|v| v.stop());

        // resend all data
        let mut idx = self.buffer.button();
        while idx != self.buffer.top() {
            // the packet is always exist
            let packet = self.buffer.get(idx).unwrap();
            buf.clear();
            let size = packet.write(buf)?;
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
