//! 滑动窗口
//! GBN

use std::time::Duration;

use tokio::{sync::oneshot, task::JoinHandle};

use crate::packet::Packet;
pub mod gbn;

pub const MAX_BUFF_SIZE: usize = 1024 * 1024 * 4 + 32;
pub const TIMEOUT_MS: u64 = 3000;
/// 定时器，用于超时重传
pub struct Timer {
    handle: JoinHandle<()>,
}

pub struct TimeoutWait(oneshot::Receiver<()>);

pub enum NeedTimeoutWork {
    Need,
    None,
}

impl TimeoutWait {
    pub async fn waiting(self) -> NeedTimeoutWork {
        match self.0.await {
            Ok(_) => NeedTimeoutWork::Need,
            // sender drop , the timer closed
            Err(_) => NeedTimeoutWork::None,
        }
    }
}

impl Timer {
    pub fn start(timeout: Duration) -> (Self, TimeoutWait) {
        let (rx, tx) = oneshot::channel();
        let handle = tokio::task::spawn(async move {
            tokio::time::sleep(timeout).await;
            rx.send(()).ok();
        });

        (Self { handle }, TimeoutWait(tx))
    }

    pub fn stop(self) {
        if !self.handle.is_finished() {
            self.handle.abort();
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[derive(Debug, Default)]
pub enum State {
    #[default]
    WaitingAck,
    Done,
}

#[derive(Debug, Default)]
pub struct StatePacket {
    pub state: State,
    pub pkg: Packet,
}

impl StatePacket {
    pub fn new_waiting(pkg: Packet) -> Self {
        Self {
            state: State::WaitingAck,
            pkg,
        }
    }

    pub fn recv_ack(&mut self) {
        self.state = State::Done
    }

    pub fn is_down(&self) -> bool {
        matches!(self.state, State::Done)
    }
}
