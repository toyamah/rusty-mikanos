use crate::error::Code;
use crate::{make_error, Error};

pub struct ArrayQueue<'a, T, const N: usize> {
    data: &'a mut [T; N],
    length: usize,
    write_pos: usize,
    read_pos: usize,
}

impl<'a, T, const N: usize> ArrayQueue<'a, T, N> {
    pub fn new(data: &'a mut [T; N]) -> Self {
        Self {
            data,
            length: 0,
            write_pos: 0,
            read_pos: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), Error> {
        if self.length >= N {
            return Err(make_error!(Code::Full));
        }

        self.data[self.write_pos] = value;
        self.length += 1;
        self.write_pos += 1;
        if self.write_pos == N {
            self.write_pos = 0
        }
        Ok(())
    }

    pub fn pop(&mut self) -> Result<&T, Error> {
        if self.length == 0 {
            return Err(make_error!(Code::Empty));
        }

        let value = &self.data[self.read_pos];
        self.length -= 1;
        self.read_pos += 1;
        if self.read_pos == N {
            self.read_pos = 0
        }
        Ok(value)
    }

    pub fn front(&self) -> &T {
        &self.data[self.read_pos]
    }

    pub fn count(&self) -> usize {
        self.length
    }
}
