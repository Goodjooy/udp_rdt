#[derive(Debug)]
pub struct FixedCycleBuffer<const S: u8, T> {
    buffer: Vec<BufferWrap<T>>,
    offset: u8,
}

impl<const S: u8, T> FixedCycleBuffer<S, T> {
    pub fn new() -> Self {
        Self {
            buffer: (0..=u8::MAX).map(|_| BufferWrap::Nil).collect(),
            offset: 0,
        }
    }

    pub fn offset(&self) -> u8 {
        self.offset
    }

    pub fn calculate_offset(&self, packet_id: u8) -> u8 {
        packet_id.wrapping_sub(self.offset)
    }

    pub fn insert(&mut self, idx: u8, data: T) -> Result<(), T> {
        if self.calculate_offset(idx) < S {
            *{ self.buffer.get_mut(idx as usize).unwrap() } = BufferWrap::Set(data);
            Ok(())
        } else {
            // local not in windows
            Err(data)
        }
    }

    pub fn slide_windows(&mut self) -> Vec<T> {
        let mut vec = Vec::new();

        loop {
            let buf = self.buffer.get_mut(self.offset as usize).unwrap();
            if buf.is_set() {
                vec.extend(buf.take());
                self.offset = self.offset.wrapping_add(1);
            } else {
                // hit nil, stop
                break vec;
            }
        }
    }
}

#[derive(Debug)]
enum BufferWrap<T> {
    Nil,
    Set(T),
}

impl<T> BufferWrap<T> {
    pub fn is_set(&self) -> bool {
        matches!(self, Self::Set(..))
    }
}

impl<T> BufferWrap<T> {
    pub fn take(&mut self) -> Option<T> {
        match std::mem::replace(self, Self::Nil) {
            BufferWrap::Nil => None,
            BufferWrap::Set(data) => Some(data),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{BufferWrap, FixedCycleBuffer};

    #[test]
    fn test() {
        let mut buffer = FixedCycleBuffer::<20, u8>::new();
        buffer.offset = 245;
        buffer
            .buffer
            .iter_mut()
            .enumerate()
            .filter(|(idx, _)| (*idx as u8).wrapping_sub(245) < 20)
            .for_each(|(idx, v)| *v = BufferWrap::Set(idx as u8));

        // slide window offset change to 9
        let v = buffer.slide_windows();

        println!("{:?}", v);
        assert_eq!(buffer.offset, 9);

        // set 10 is ok
        let v = buffer.insert(10, 10);
        assert_eq!(v, Ok(()));
        // set 28 is ok
        let v = buffer.insert(28, 28);
        assert_eq!(v, Ok(()));

        // cannot slide
        let v = buffer.slide_windows();
        assert_eq!(v.len(), 0);
        assert_eq!(buffer.offset, 9);

        // set 8 is bad
        let err = buffer.insert(8, 8);
        assert_eq!(err, Err(8));

        // but set 9 is ok
        let ok = buffer.insert(9, 9);
        assert_eq!(ok, Ok(()));

        // if slide this time ,will side to 11
        // 9, 10 return
        let v = buffer.slide_windows();

        assert_eq!(v, [9, 10]);
        assert_eq!(buffer.offset, 11);
    }
}
