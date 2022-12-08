use crate::{ProtocolCursor, ProtocolError, ProtocolResult};

#[derive(Clone, Copy)]
pub struct SliceProtocolCursor<'a> {
    pub slice: &'a [u8],
    pub current: usize,
}

impl<'a> SliceProtocolCursor<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self { slice, current: 0 }
    }
}

impl<'a> ProtocolCursor<'a> for SliceProtocolCursor<'a> {
    fn take_byte(&mut self) -> ProtocolResult<u8> {
        match self.current == self.slice.len() {
            true => Err(ProtocolError::End),
            false => {
                let byte = self.slice[self.current];
                self.current += 1;
                Ok(byte)
            }
        }
    }

    fn take_bytes(&mut self, length: usize) -> ProtocolResult<&'a [u8]> {
        match self.has_bytes(length) {
            true => {
                let slice = &self.slice[self.current..(length + self.current)];
                self.current += length;
                Ok(slice)
            },
            false => Err(ProtocolError::End),
        }
    }

    fn remaining_bytes(&self) -> usize {
        self.slice.len() - self.current
    }
}