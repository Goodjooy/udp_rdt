//! client
//! send msg to server

use std::{net::Ipv4Addr, time::Duration, io::ErrorKind};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    time::timeout,
};
use udp_rdt::{packet::{ack::Ack, Packet}, fake_udp::UdpSocket};

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(task())
}

async fn task() {
    let udp_socket = UdpSocket::bind((Ipv4Addr::from([127, 0, 0, 1]), 5000))
        .await
        .expect("Create UDP Socket Failure");

    println!("UDP Client Started");

    let target_addr = (Ipv4Addr::from([127, 0, 0, 1]), 8080);

    let mut buf = [0u8; 1024 + 128];
    let mut write_buf = Vec::with_capacity(1024 + 128);
    let mut local_id = 0u8;
    let mut state = State::WaitMsg;
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut in_string = String::new();
    loop {
        match state {
            // read msg from input
            State::WaitMsg => {
                in_string.clear();
                let size = stdin
                    .read_line(&mut in_string)
                    .await
                    .expect("Read string form stdin failure");

                in_string = in_string.trim().to_owned();
                if size > 1024 {
                    eprintln!("Msg size must < 1024 ,but get {size}")
                }

                println!("Get STDIN string {:?}", in_string);

                // generate send packet
                let packet = Packet::new_data(local_id, in_string.as_bytes().to_owned());
                write_buf.clear();
                let size = packet.write(&mut write_buf).expect("Send body Over flow");
                let send_body = &write_buf[0..size];

                println!("send body {:?}", send_body);

                udp_socket
                    .send_to(send_body, target_addr)
                    .await
                    .expect("Cannot send Body");

                println!("Send string {:?} Down, waiting ACK", in_string);

                // update state,waiting for resp
                state = State::WaitAck;
            }
            State::WaitAck => {
                let result =
                    timeout(Duration::from_millis(5000), udp_socket.recv_from(&mut buf)).await;

                match result {
                    Ok(Ok((size, _))) => {
                        println!("Recv Might ACK");

                        let body = &buf[0..size];
                        let ack = Ack::read(body);
                        if let Ok(Some(ack)) = ack {
                            if ack.is_correct_ack(local_id) {
                                println!("ACK Pass!");
                                // 接收确认OK, 等待下一次输入
                                state = State::WaitMsg;
                                // update local id
                                local_id = local_id.wrapping_add(1);
                                continue;
                            }
                        }
                    }
                    Ok(Err(ref err)) => {
                        let kind = err.kind();
                        println!("Error kind {kind:?}");
                        if let ErrorKind::ConnectionReset = kind{
                            eprintln!("Service not usable");
                            break;
                        }
                    }
                    _ => {}
                }

                println!("ACK Failure , Send again");
                // not rev ack
                // 1 time out
                // 2 bad ACK
                // 3 get previous ack (equal to NAK)
                // need send packet again
                let packet = Packet::new_data(local_id, in_string.as_bytes().to_owned());
                write_buf.clear();
                let size = packet.write(&mut write_buf).expect("Send Body Over flow");
                let send_body = &write_buf[0..size];
                udp_socket
                    .send_to(send_body, target_addr)
                    .await
                    .expect("Cannot Send Body");
            }
        }
    }
}

enum State {
    WaitMsg,
    WaitAck,
}
