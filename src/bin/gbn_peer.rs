#![feature(box_syntax)]
use std::collections::VecDeque;
use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

use clap::Parser;
use futures::future::Either;
use tokio::io::AsyncBufReadExt;

use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::task;
use tokio::{sync::mpsc, task::LocalSet};
use udp_rdt::{
    fake_udp::UdpSocket,
    packet::Packet,
    slide_windows::{
        gbn::{start_receive_peer, start_send_peer, RecvMsg, SenderMsg},
        MAX_BUFF_SIZE,
    },
};

fn main() {
    let args = Args::parse();
    println!("Arg : {args:?}");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Start Rt Fault");

    rt.block_on(task(args))
}

async fn task(args: Args) {
    let mut map = BTreeMap::new();
    let socket = Arc::new(
        UdpSocket::bind(args.local_addr)
            .await
            .expect("Cannot Create Udp socket"),
    );

    let local_set = LocalSet::new();
    let (timeout_rx, mut timeout_tx) = mpsc::channel(16);
    let (output_rx, mut output_tx) = mpsc::channel::<Vec<u8>>(10);

    let send_msg = start_send_peer(Arc::clone(&socket), args.target_addr, timeout_rx.clone());
    let recv = start_receive_peer(Arc::clone(&socket), args.target_addr, output_rx.clone());
    map.insert(args.target_addr, recv);

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

    let input_sender = send_msg.clone();
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
                    input_sender.send(SenderMsg::Msg(s.into_bytes())).await.ok();
                }
            }
        }
    });

    let mut buf = box [0u8; MAX_BUFF_SIZE];
    local_set
        .run_until(async move {
            loop {
                let task = match futures::future::select(
                    Box::pin(socket.recv_from(buf.as_mut())),
                    Box::pin(timeout_tx.recv()),
                )
                .await
                {
                    Either::Left((r, _)) => Either::Left(r),
                    Either::Right((l, _)) => Either::Right(l),
                };

                match task {
                    Either::Left(r) => {
                        let (size, origin) = r.expect("Udp Socket Fault");
                        let body = &buf[0..size];
                        if origin == args.target_addr {
                            let packet = Packet::read(body);
                            if let Ok(Some(packet)) = packet {
                                eprintln!("Recv Might ACK packet Ok");
                                if packet.is_ack() {
                                    eprintln!("Recv ACK {}", packet.get_ack_num());
                                    send_msg
                                        .send(SenderMsg::Ack(packet.get_ack_num()))
                                        .await
                                        .expect("Failure Handle Msg");

                                    continue;
                                }
                            }
                        }
                        // else
                        // if fault ack packet , recv not reaction
                        // if peer send msg , handle it
                        if let Some(sender) = map.get(&origin) {
                            // origin socket send previous
                            sender.send(RecvMsg(body.to_vec())).await.ok();
                        } else {
                            // new origin start recv
                            let sender =
                                start_receive_peer(Arc::clone(&socket), origin, output_rx.clone());
                            sender.send(RecvMsg(body.to_vec())).await.ok();
                            map.insert(origin, sender);
                        }
                    }
                    Either::Right(Some(_)) => {
                        eprintln!("Time Out Resend all");
                        send_msg.send(SenderMsg::ResendAll).await.ok();
                    }
                    Either::Right(None) => (),
                }
            }
        })
        .await;
}

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, short, value_parser)]
    local_addr: SocketAddr,
    #[clap(long, short, value_parser)]
    target_addr: SocketAddr,
}
