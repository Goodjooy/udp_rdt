//! Go back N
//!
//!

mod receiver;
use std::{io, net::SocketAddr, sync::Arc};
mod sender;

#[derive(Debug, thiserror::Error)]
pub enum GbnError {
    #[error("Io Error {0}")]
    Io(#[from] io::Error),

    #[error(transparent)]
    CycleBuffer(#[from]CbError),
    /// 缓冲区已满
    #[error("缓冲区已满")]
    BufferFilled,
    #[error("Packet 校验错误")]
    PacketFault,

    #[error("Packet ID 不匹配")]
    PacketIdMisMatch,
}

pub use receiver::GoBackNReceiver;
pub use sender::GoBackNSender;
use tokio::sync::mpsc;

use crate::{fake_udp::UdpSocket, packet::Packet, cycle_buffer::CbError};

use super::MAX_BUFF_SIZE;

#[derive(Debug)]
pub enum SenderMsg {
    Msg(Vec<u8>),
    Ack(u8),
    ResendAll,
}

pub fn start_send_peer(
    socket: Arc<UdpSocket>,
    target: SocketAddr,
    timeout_send: mpsc::Sender<()>,
) -> mpsc::Sender<SenderMsg> {
    let (rx, mut tx) = mpsc::channel(128);
    let mut sender = GoBackNSender::new(target);
    let task = async move {
        let mut write_buf = Vec::with_capacity(MAX_BUFF_SIZE);
        while let Some(msg) = tx.recv().await {
            let result = async {
                match msg {
                    SenderMsg::Ack(ack) => {
                        sender.recv_ack(ack, timeout_send.clone()).await;
                        Ok(())
                    }
                    SenderMsg::ResendAll => {
                        sender
                            .resend_all(&mut write_buf, &socket, timeout_send.clone())
                            .await
                    }
                    SenderMsg::Msg(msg) => {
                        sender
                            .send(&mut write_buf, msg, &socket, timeout_send.clone())
                            .await
                    }
                }
            };

            match result.await {
                Ok(_) => (),
                Err(err) => {
                    eprintln!("Error 发生 {err}");
                }
            }
        }
    };

    tokio::spawn(task);
    rx
}

pub struct RecvMsg(pub Vec<u8>);

pub fn start_receive_peer(
    socket: Arc<UdpSocket>,
    origin: SocketAddr,
    output: mpsc::Sender<Vec<u8>>,
) -> mpsc::Sender<RecvMsg> {
    let (rx, mut tx) = mpsc::channel(1);
    let mut receiver = GoBackNReceiver::new(origin);

    let task = async move {
        let mut write_buf = Vec::with_capacity(MAX_BUFF_SIZE);
        while let Some(RecvMsg(buf)) = tx.recv().await {
            let result = async {
                let packet = Packet::read(buf.as_slice());
                if let Ok(Some(packet)) = packet {
                    if packet.is_data() {
                        let v = receiver.receive(&mut write_buf, packet, &socket).await?;
                        output.send(v).await.ok();
                    }
                }
                Result::<_, GbnError>::Ok(())
            };

            match result.await {
                Ok(_) => (),
                Err(err) => {
                    match err {
                        GbnError::PacketFault | GbnError::PacketIdMisMatch => {
                            receiver.send_ack(&mut write_buf, &socket).await.ok();
                        }
                        _ => (),
                    }
                    eprintln!("Error 发生 {err}")
                }
            }
        }
    };

    tokio::task::spawn(task);

    rx
}
