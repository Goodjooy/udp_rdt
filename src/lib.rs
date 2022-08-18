use std::{collections::VecDeque, net::SocketAddr};

use clap::Parser;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::mpsc,
    task,
};

pub mod cycle_buffer;
pub mod fake_udp;
pub mod fixed_cycle_buf;
pub mod packet;
pub mod slide_windows;
pub mod verify;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long, short, value_parser)]
    pub local_addr: SocketAddr,
    #[clap(long, short, value_parser)]
    pub target_addr: SocketAddr,
}

pub fn start_output() -> mpsc::Sender<Vec<u8>> {
    let (output_rx, mut output_tx) = mpsc::channel::<Vec<u8>>(64);

    task::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(s) = output_tx.recv().await {
            let s = String::from_utf8_lossy(&s);

            stdout
                .write_all(format!("\n-----------\nrecv Msg \n{:?} \n-----------\n", s).as_bytes())
                .await
                .expect("Write fault");
        }
    });

    output_rx
}

pub fn start_input<T, F>(sender: mpsc::Sender<T>, handle: F)
where
    T: Send + 'static,
    F: Fn(String) -> T + Send + 'static,
{
    task::spawn(async move {
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();
        let mut strings = VecDeque::new();
        while let Ok(Some(s)) = lines.next_line().await {
            let s = s.trim().to_string();
            println!("input msg :{:?}", s);
            if !s.is_empty() {
                strings.push_back(s);
            } else {
                while let Some(s) = strings.pop_front() {
                    let data = handle(s);
                    sender.send(data).await.ok();
                }
            }
        }
    });
}
