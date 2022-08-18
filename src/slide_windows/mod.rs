//! 滑动窗口

use std::time::Duration;

use futures::Future;
use tokio::{sync::oneshot, task::JoinHandle};

use crate::packet::Packet;
pub mod gbn;
pub mod sr;

pub const MAX_BUFF_SIZE: usize = 1024 * 1024 * 4 + 32;
pub const TIMEOUT_MS: u64 = 5000;
/// 定时器，用于超时重传
pub struct Timer {
    handle: JoinHandle<()>,
}

pub struct TimeoutWait(oneshot::Receiver<()>);

pub struct TimerStarter(oneshot::Sender<()>);

impl TimerStarter {
    pub fn start(self) {
        self.0.send(()).ok();
    }
}

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

    pub fn need_resend_do<F>(self, fut: F)
    where
        F: Future<Output = ()> + 'static + Send,
    {
        tokio::task::spawn(async move {
            match self.waiting().await {
                NeedTimeoutWork::Need => {
                    fut.await;
                }
                NeedTimeoutWork::None => (),
            }
        });
    }
}

impl Timer {
    pub fn later_start(timeout: Duration) -> (Self, TimerStarter, TimeoutWait) {
        let (start_rx, start_tx) = oneshot::channel();
        let (rx, tx) = oneshot::channel();
        let handle = tokio::task::spawn(async move {
            match start_tx.await {
                Ok(_) => {
                    tokio::time::sleep(timeout).await;
                    rx.send(()).ok();
                }
                Err(_) => todo!(),
            }
        });

        (Self { handle }, TimerStarter(start_rx), TimeoutWait(tx))
    }

    pub fn start(timeout: Duration) -> (Self, TimeoutWait) {
        let (rx, tx) = oneshot::channel();
        let handle = tokio::task::spawn(async move {
            tokio::time::sleep(timeout).await;
            rx.send(()).ok();
        });

        (Self { handle }, TimeoutWait(tx))
    }

    pub fn stop(&self) {
        if !self.handle.is_finished() {
            self.handle.abort();
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.stop();
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
