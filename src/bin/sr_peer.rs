#![feature(box_syntax)]

use std::{collections::BTreeMap, sync::Arc};

use clap::Parser;
use futures::future::{select, Either};
use tokio::sync::mpsc;
use udp_rdt::{
    fake_udp::UdpSocket,
    packet::Packet,
    slide_windows::{
        sr::{start_receive_peer, start_send_peer, RecvMsg, SenderMsg},
        MAX_BUFF_SIZE,
    },
    start_input, start_output, Args,
};

fn main() {
    let args = Args::parse();
    println!("Args {args:?}");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Start time out fault");

    rt.block_on(task(args));
}

async fn task(
    Args {
        local_addr,
        target_addr,
    }: Args,
) {
    let mut peers = BTreeMap::new();

    let socket = Arc::new(
        UdpSocket::bind(local_addr)
            .await
            .expect("Start Udp Socket Failure"),
    );

    let (timeout_rt, mut timeout_tx) = mpsc::channel::<u8>(8);
    let output_send = start_output();
    let send_msg = start_send_peer(Arc::clone(&socket), target_addr, timeout_rt.clone());
    let recv = start_receive_peer(Arc::clone(&socket), target_addr, output_send.clone());
    peers.insert(target_addr, recv);

    start_input(send_msg.clone(), |s| SenderMsg::Msg(s.into_bytes()));

    let mut buf = box [0u8; MAX_BUFF_SIZE];

    loop {
        let task = match select(
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
                let (size, origin) = r.expect("Recv Udp Failure");
                let body = &&buf[0..size];
                let packet = Packet::read(&body);

                if origin == target_addr {
                    if let Ok(Some(ref packet)) = packet {
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
                if let Some(sender) = peers.get(&origin) {
                    sender.send(RecvMsg(packet)).await.ok();
                } else {
                    let sender =
                        start_receive_peer(Arc::clone(&socket), origin, output_send.clone());
                    sender.send(RecvMsg(packet)).await.ok();
                    peers.insert(origin, sender);
                }
            }
            Either::Right(Some(ack)) => {
                eprintln!("Time Out Resend {ack}");
                send_msg.send(SenderMsg::Resend(ack)).await.ok();
            }
            Either::Right(None) => {}
        }
    }
}
