//! Select Resend
//! 选择重传
//!
//!
mod receiver;
mod sender;
const MAX_WINDOWS_SIZE: u8 = 128;
use std::io;

pub use receiver::SelectResendReceiver;
pub use sender::SelectResendSender;

use crate::cycle_buffer::CbError;

#[derive(Debug, thiserror::Error)]
pub enum SrError {
    #[error("IO 异常 {0}")]
    Io(#[from] io::Error),
    #[error(transparent)]
    CycleBuffer(#[from] CbError),
}
