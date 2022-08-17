//! service
//! recv the message comes form client

use std::{net::Ipv4Addr, time::Duration};

use udp_rdt::{
    fake_udp::UdpSocket,
    packet::{ack::Ack, Packet},
};

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(task())
}

async fn task() {
    let udp_socket = UdpSocket::bind((Ipv4Addr::from([127, 0, 0, 1]), 8080))
        .await
        .unwrap();

    println!("UDP Server Started!");

    let mut buf = [0u8; 1024 + 128];
    let mut write_buf = Vec::with_capacity(1024 + 128);
    let mut local_id = 0u8;
    let mut last_ack = Ack::new_ack(u8::MAX);

    while let Ok((size, origin)) = udp_socket.recv_from(&mut buf).await {
        let local_buf = &buf[0..size];
        println!("Recv Packet Size : [{size}]");
        println!("Recv body {:?}", local_buf);
        let packet = Packet::read(local_buf);
        if let Ok(Some(packet)) = packet {
            println!("get Packet {:?}", packet);

            if packet.get_id() == local_id {
                println!("Packet Verify Pass");
                // ok
                // send ack
                last_ack = Ack::new_ack(local_id);
                // clear write buf
                write_buf.clear();
                let size = last_ack.write(&mut write_buf).expect("send body over flow");
                let send_body = &write_buf[0..size];
                udp_socket
                    .send_to(send_body, origin)
                    .await
                    .expect("cannot send ACK");

                // update local id
                local_id = local_id.wrapping_add(1);

                // show msg
                let body = packet.get_body();
                let body = String::from_utf8_lossy(&body);
                println!("{body}");

                continue;
            }
        }
        println!(
            "Packet Verify not pass, send last ACK {}",
            last_ack.get_ack_num()
        );
        // error or send same packet again
        // send last ack
        write_buf.clear();
        let size = last_ack.write(&mut write_buf).expect("send buf over flow");
        let body = &write_buf[0..size];
        udp_socket
            .send_to(body, origin)
            .await
            .expect("cannot send ACK");

        println!("Something wrong on Packet");

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
