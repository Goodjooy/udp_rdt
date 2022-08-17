//! Go back N
//!
//!

use std::io;
mod sender;

#[derive(Debug, thiserror::Error)]
pub enum GbnError {
    #[error("Io Error {0}")]
    Io(#[from] io::Error),
    /// 缓冲区已满
    #[error("缓冲区已满")]
    BufferFilled,
}
