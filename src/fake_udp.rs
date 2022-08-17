//! Fake Udp
//! 由于本地测试中即使是UDP也能可靠传输，因此加入人为随机数来进行模拟随机的
//! 分组丢失
//! bit 反转

use std::{io, net::SocketAddr};

use rand::Rng;
use tokio::net::{self, ToSocketAddrs};

pub struct UdpSocket {
    inner: net::UdpSocket,
}

impl UdpSocket {
    pub async fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let inner = net::UdpSocket::bind(addr).await?;
        Ok(Self { inner })
    }

    pub async fn send_to(&self, data: &[u8], target: impl ToSocketAddrs) -> io::Result<usize> {
        let mut rand = rand::rngs::OsRng;

        // 随机丢失 20%丢包率
        if rand.gen_bool(0.2) {
            println!("Packet Loss");
            return Ok(data.len());
        }
        let mut data = data.to_owned();
        // 随机byte 变换
        if rand.gen_bool(0.2) {
            println!("Packet Mistake");
            let pos_num = rand.gen_range(0..10);

            for _ in 0..pos_num {
                let idx = rand.gen_range(0..data.len());
                let value = rand.gen();
                data[idx] = value;
            }
        }

        self.inner.send_to(&data, target).await
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.inner.recv_from(buf).await
    }
}
