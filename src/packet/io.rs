use std::io::{self, copy, Cursor, Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt, BE};

use crate::verify::{verify, verify_info_gen};

use super::{
    flags::{BodySize, PacketFlag},
    Packet,
};

impl Packet {
    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let flag =
            PacketFlag::from_packet_info(self.body.len(), self.packet_split, self.packet_type);
        let body = &mut self.body.as_slice();
        let mut buf_writer = Vec::with_capacity(1 + self.body.len() + 2);

        // generate send body
        // body flag
        buf_writer.write_u8(flag.get_flag())?;
        // body code
        buf_writer.write_u8(self.identify_code)?;
        // body size
        let _ = Self::write_body_size(body.len(), flag, writer)?;
        // body
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
            // pack flag
            let flag = PacketFlag::from_reader(&mut reader)?;

            // packet code
            let code = reader.read_u8()?;
            // packet size
            let size = Self::read_body_size(&flag, &mut reader)?;

            let ty = flag.get_pack_type();
            let split = flag.get_pack_split();

            // body
            let body = size
                .map(|size| {
                    let mut buf = vec![0u8; size];
                    reader.read_exact(&mut buf).map(|_| buf)
                })
                .transpose()?;

            // verify info
            let _ = reader.read_u16::<BE>()?;

            let task = || Some(Self::new(code, body?, ty?, split?));

            Ok(task())
        }
    }

    fn write_body_size<W: Write>(
        size: usize,
        flag: PacketFlag,
        writer: &mut W,
    ) -> io::Result<usize> {
        match flag.get_pack_size().unwrap() {
            BodySize::U64 => {
                writer.write_u64::<BE>(size as _)?;
                Ok(8)
            }
            BodySize::U32 => {
                writer.write_u32::<BE>(size as _)?;
                Ok(4)
            }
            BodySize::U16 => {
                writer.write_u16::<BE>(size as _)?;
                Ok(2)
            }
            BodySize::U8 => {
                writer.write_u8(size as _)?;
                Ok(1)
            }
            BodySize::Single | BodySize::Empty => Ok(0),
        }
    }

    fn read_body_size<R: Read>(flag: &PacketFlag, reader: &mut R) -> io::Result<Option<usize>> {
        flag.get_pack_size()
            .map(|size| match size {
                BodySize::U64 => reader.read_u64::<BE>().map(|v| v as _),
                BodySize::U32 => reader.read_u32::<BE>().map(|v| v as _),
                BodySize::U16 => reader.read_u16::<BE>().map(|v| v as _),
                BodySize::U8 => reader.read_u8().map(|v| v as _),
                BodySize::Single => Ok(1usize),
                BodySize::Empty => Ok(0),
            })
            .transpose()
    }
}
