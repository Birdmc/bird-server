use std::ops::BitOrAssign;
use crate::{ProtocolCursor, ProtocolError, ProtocolReadable, ProtocolResult, ProtocolWritable, ProtocolWriter};

impl<'a> ProtocolCursor<'a> for &'a [u8] {
    fn take_byte(&mut self) -> ProtocolResult<u8> {
        match self.remaining_bytes() == 0 {
            true => Err(ProtocolError::End),
            false => {
                let byte = self[0];
                *self = &self[1..];
                Ok(byte)
            }
        }
    }

    fn take_bytes(&mut self, length: usize) -> ProtocolResult<&'a [u8]> {
        match self.has_bytes(length) {
            true => {
                let slice = &self[0..length];
                *self = &self[length..];
                Ok(slice)
            },
            false => Err(ProtocolError::End),
        }
    }

    fn remaining_bytes(&self) -> usize {
        self.len()
    }
}

impl ProtocolWriter for Vec<u8> {
    fn write_bytes(&mut self, bytes: &[u8]) {
        // TODO change implementation
        for byte in bytes {
            self.push(*byte)
        }
    }
}