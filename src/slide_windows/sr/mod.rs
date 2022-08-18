//! Select Resend
//! 选择重传
//!
//!
mod receiver;
mod sender;
const MAX_WINDOWS_SIZE: u8 = 128;
use std::{io, net::SocketAddr, sync::Arc};

pub use receiver::SelectResendReceiver;
pub use sender::SelectResendSender;
use tokio::sync::mpsc;

use crate::{cycle_buffer::CbError, fake_udp::UdpSocket, packet::Packet};

use super::MAX_BUFF_SIZE;

#[derive(Debug, thiserror::Error)]
pub enum SrError {
    #[error("IO 异常 {0}")]
    Io(#[from] io::Error),
    #[error(transparent)]
    CycleBuffer(#[from] CbError),
}

#[derive(Debug)]
pub enum SenderMsg {
    Msg(Vec<u8>),
    Ack(u8),
    Resend(u8),
}

pub fn start_send_peer(
    socket: Arc<UdpSocket>,
    target: SocketAddr,
    timeout_send: mpsc::Sender<u8>,
) -> mpsc::Sender<SenderMsg> {
    let (rx, mut tx) = mpsc::channel(128);
    let mut sender = SelectResendSender::new(target);

    let task = async move {
        let mut write_buf = Vec::with_capacity(MAX_BUFF_SIZE);

        while let Some(msg) = tx.recv().await {
            let result = async {
                match msg {
                    SenderMsg::Msg(msg) => {
                        sender
                            .send(&mut write_buf, msg, &socket, timeout_send.clone())
                            .await
                    }
                    SenderMsg::Ack(ack) => {
                        sender.recv_ack(ack).await;
                        Ok(())
                    }
                    SenderMsg::Resend(packet_id) => {
                        sender
                            .select_resend(packet_id, &mut write_buf, &socket, timeout_send.clone())
                            .await
                    }
                }
            };

            match result.await {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("Send Error {}", err)
                }
            }
        }
    };

    tokio::spawn(task);

    rx
}

pub struct RecvMsg(pub io::Result<Option<Packet>>);

pub fn start_receive_peer(
    socket: Arc<UdpSocket>,
    origin: SocketAddr,
    output: mpsc::Sender<Vec<u8>>,
) -> mpsc::Sender<RecvMsg> {
    let (rx, mut tx) = mpsc::channel(128);
    let mut receiver = SelectResendReceiver::new(origin);

    let task = async move {
        let mut write_buf = Vec::with_capacity(MAX_BUFF_SIZE);
        while let Some(RecvMsg(packet)) = tx.recv().await {
            let result = async {
                if let Ok(Some(packet)) = packet {
                    if packet.is_data() {
                        let recv = receiver.receive(&mut write_buf, packet, &socket).await?;
                        for vec in recv {
                            output.send(vec).await.ok();
                        }
                    }
                }
                Result::<_, SrError>::Ok(())
            };

            match result.await {
                Ok(_) => (),
                Err(err) => {
                    eprintln!("Recv Error {err}")
                }
            }
        }
    };

    tokio::spawn(task);

    rx
}
