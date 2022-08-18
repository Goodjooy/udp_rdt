pub struct CycleBuffer<const S: u8, T> {
    buffer: Vec<BufferWrap<T>>,
    size: u8,
    top: u8,
    button: u8,
}

impl<const S: u8, T> CycleBuffer<S, T> {
    pub fn get(&self, buf_id: u8) -> Option<&T> {
        match self.buffer.get(buf_id as usize).unwrap() {
            BufferWrap::Data(d, BufferState::Waiting) => Some(d.as_ref()),
            _ => None,
        }
    }

    pub fn len(&self) -> u8 {
        self.size
    }
    pub fn button(&self) -> u8 {
        self.button
    }

    pub fn top(&self) -> u8 {
        self.top
    }

    pub fn push(&mut self, data: T) -> Result<(), CbError> {
        if self.size == S {
            Err(CbError::BufferFilled)?
        }

        self.buffer.get_mut(self.top as usize).unwrap().update(data);
        self.top = self.top.wrapping_add(1);
        self.size += 1;

        Ok(())
    }

    pub fn buffer_down(&mut self, buf_id: u8) {
        self.buffer.get_mut(buf_id as usize).unwrap().set_down();
    }

    pub fn slide_buff(&mut self) {
        let mut idx = self.button;

        while idx != self.top {
            let buf = self.buffer.get_mut(idx as usize).unwrap();
            if buf.is_down() {
                buf.remove();
                self.size -= 1;
            } else {
                break;
            }

            idx = idx.wrapping_add(1);
            self.button = idx;
        }
    }

    pub fn set_button(&mut self, buf_id: u8) -> Result<(), ()> {
        // buf id in windows , update
        if buf_id.wrapping_sub(self.button) < self.top.wrapping_sub(self.button) {
            // 当前buf id 以及之前的均完成了
            self.size -= buf_id.wrapping_sub(self.button).wrapping_add(1);
            self.button = buf_id.wrapping_add(1);
            Ok(())
        } else {
            // do nothing
            Err(())
        }
    }
}

impl<const S: u8, T> CycleBuffer<S, T> {
    pub fn new() -> Self {
        Self {
            buffer: (0..=u8::MAX).map(|_| BufferWrap::Nil).collect(),
            size: 0,
            top: 0,
            button: 0,
        }
    }
}

#[derive(Debug, Default)]
pub enum BufferWrap<T> {
    #[default]
    Nil,
    Data(Box<T>, BufferState),
}

impl<T> BufferWrap<T> {
    #[inline]
    pub fn update(&mut self, data: T) {
        *self = BufferWrap::Data(Box::new(data), BufferState::Waiting);
    }

    pub fn remove(&mut self) {
        *self = BufferWrap::Nil;
    }

    pub fn set_down(&mut self) {
        match self {
            BufferWrap::Nil => todo!(),
            BufferWrap::Data(_, d) => *d = BufferState::Done,
        }
    }

    pub fn is_down(&self) -> bool {
        match self {
            BufferWrap::Data(_, BufferState::Done) => true,
            _ => false,
        }
    }
}
#[derive(Debug, Default)]
pub enum BufferState {
    Done,
    #[default]
    Waiting,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CbError {
    #[error("缓冲区已满")]
    BufferFilled,
}

#[cfg(test)]
mod test {
    use crate::cycle_buffer::{BufferState, BufferWrap};

    use super::{CbError, CycleBuffer};

    #[test]
    fn test_slide() {
        let mut buf = CycleBuffer::<16, u8>::new();
        buf.buffer
            .iter_mut()
            .enumerate()
            .filter(|(idx, _)| idx < &5 || idx >= &245)
            .for_each(|(_, v)| *v = BufferWrap::Data(Box::new(11), BufferState::Done));
        buf.button = 245;
        buf.top = 5;
        buf.size = 16;

        // buffer filled Error
        let resp = buf.push(11);

        assert_eq!(resp, Err(CbError::BufferFilled));

        //set a buf id out of top and button range , nothing happen
        buf.set_button(200).ok();

        assert_eq!(buf.button, 245);
        assert_eq!(buf.top, 5);
        assert_eq!(buf.size, 16);

        // set a buf id in top and button range , update button
        buf.set_button(250).ok();

        assert_eq!(buf.button, 251);
        assert_eq!(buf.top, 5);
        assert_eq!(buf.size, 10);

        // now insert data ,update top
        let resp = buf.push(112);

        assert_eq!(resp, Ok(()));
        assert_eq!(buf.button, 251);
        assert_eq!(buf.top, 6);
        assert_eq!(buf.size, 11);

        // slide windows , size become 0
        buf.slide_buff();

        assert_eq!(buf.button, 5);
        assert_eq!(buf.top, 6);
        assert_eq!(buf.size, 1);

        // task down
        buf.buffer_down(buf.button);
        buf.slide_buff();
        assert_eq!(buf.button, 6);
        assert_eq!(buf.top, 6);
        assert_eq!(buf.size, 0);
    }
}
