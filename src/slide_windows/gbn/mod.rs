//! Go back N
//!
//!

mod receiver;
use std::io;
mod sender;

#[derive(Debug, thiserror::Error)]
pub enum GbnError {
    #[error("Io Error {0}")]
    Io(#[from] io::Error),
    /// 缓冲区已满
    #[error("缓冲区已满")]
    BufferFilled,
    #[error("Packet 校验错误")]
    PacketFault,

    #[error("Packet ID 不匹配")]
    PacketIdMisMatch,
}

pub use sender::GoBackNSender;
pub use receiver::GoBackNReceiver;