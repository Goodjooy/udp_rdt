//! verify code of transform body

use byteorder::{ReadBytesExt, BE};

pub fn verify_info_gen(buf: &[u8]) -> u16 {
    let mut start = 0u16;
    let mut reader = buf;

    // 缺位补0
    // 读取1个u8
    if buf.len() % 2 != 0 {
        let num = reader.read_u8().unwrap();
        start += num as u16;
    }

    (0..(buf.len() / 2)).for_each(|_| {
        let num = reader.read_u16::<BE>().unwrap();

        let (r, is_overflow) = start.overflowing_add(num);
        if is_overflow {
            start = r + 1;
        } else {
            start = r;
        }
    });

    !start
}

pub fn verify(data: &[u8]) -> bool {
    let size = data.len();
    let mut verify = 0u16;
    let mut reader = data;

    if size % 2 != 0 {
        let num = reader.read_u8().unwrap();
        verify += num as u16;
    }

    (0..(size / 2)).for_each(|_| {
        let num = reader.read_u16::<BE>().unwrap();

        let (r, is_overflow) = verify.overflowing_add(num);
        verify = r + if is_overflow { 1 } else { 0 };
    });

    verify == u16::MAX
}

#[cfg(test)]
mod test {
    use byteorder::{WriteBytesExt, BE};
    use rand::RngCore;

    use super::{verify, verify_info_gen};

    #[test]
    fn test_gen_verify() {
        let mut data = b"1234567".to_vec();
        let info = verify_info_gen(&data);

        println!("{info:b}");

        data.write_u16::<BE>(info).unwrap();

        assert!(verify(&data))
    }

    fn rand_test() {
        let mut rand = rand::thread_rng();

        let mut buf = vec![0u8; 512];
        rand.fill_bytes(&mut buf);

        let verify_data = verify_info_gen(&buf);
        println!("{verify_data:016b}");

        buf.write_u16::<BE>(verify_data).unwrap();

        assert!(verify(&buf))
    }

    #[test]
    fn test_many_random() {
        for _ in 0..1024 {
            rand_test()
        }
    }
    #[test]
    fn test_bad() {
        let data = b"1234567".to_vec();
        let mut data2 = b"1234568".to_vec();
        let info = verify_info_gen(&data);

        println!("{info:b}");

        data2.write_u16::<BE>(info).unwrap();

        assert!(!verify(&data2))
    }
}
