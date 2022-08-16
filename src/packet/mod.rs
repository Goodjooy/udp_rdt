pub mod ack;
use std::io::{self, copy, Cursor, Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt, BE};

use crate::verify::{verify, verify_info_gen};

#[derive(Debug)]
pub struct Packet {
    /// 差错校验码
    // verify: u16,
    /// 当前packet 的编号
    /// 0 ~ 256
    identify_code: u8,
    /// body ,assume the body size <= 512
    body: Vec<u8>,
}

impl Packet {
    pub fn new(code: u8, body: Vec<u8>) -> Self {
        Self {
            identify_code: code,
            body,
        }
    }

    pub fn get_body(self) -> Vec<u8> {
        self.body
    }
    pub fn get_id(&self) -> u8 {
        self.identify_code
    }
}

impl Packet {
    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let body = &mut self.body.as_slice();
        let mut buf_writer = Vec::with_capacity(1 + self.body.len() + 2);

        // generate send body
        buf_writer.write_u8(self.identify_code)?;
        buf_writer.write_u32::<BE>(self.body.len() as u32)?;
        copy(body, &mut buf_writer)?;

        // generate verify code
        let verify = verify_info_gen(&buf_writer);

        // write body
        let mut buf_writer = Cursor::new(buf_writer);
        let size = copy(&mut buf_writer, writer)?;
        // write verify
        writer.write_u16::<BE>(verify)?;

        Ok(2 + size as usize)
    }

    pub fn read(entity: &[u8]) -> io::Result<Option<Self>> {
        if !verify(entity) {
            println!("Verify Failure");
            Ok(None)
        } else {
            let mut reader = entity;
            // packet code
            let code = reader.read_u8()?;
            // packet size
            let size = reader.read_u32::<BE>()?;

            let mut buf = vec![0u8; size as usize];
            reader.read_exact(&mut buf)?;

            // verify info
            let _ = reader.read_u16::<BE>()?;

            Ok(Some(Self::new(code, buf)))
        }
    }
}
