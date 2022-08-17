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
        let buf_writer = {
            let mut writer = Vec::with_capacity(1 + self.body.len() + 2);

            // generate send body
            // body flag
            writer.write_u8(flag.get_flag())?;
            // body code
            writer.write_u8(self.identify_code)?;
            // body size

            let _ = Self::write_body_size(body.len(), flag, &mut writer)?;
            // body
            copy(body, &mut writer)?;

            writer
        };
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

#[cfg(test)]
mod test {
    use crate::packet::{flags::PacketFlag, Packet};

    #[test]
    fn test_write_body_size() {
        let mut buf = Vec::new();

        let flag = PacketFlag::from_packet_info(3, Default::default(), Default::default());

        Packet::write_body_size(3, flag, &mut buf).unwrap();

        println!("buf {:?}", buf);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 3);
    }

    #[test]
    fn test_self_verify() {
        let packet = Packet::new_data(0, vec![1, 1, 1]);
        let mut buf = Vec::new();

        packet.write(&mut buf).unwrap();
        println!("buf :{buf:?}");

        let resp = Packet::read(&buf);

        println!("{resp:?}")
    }
}
