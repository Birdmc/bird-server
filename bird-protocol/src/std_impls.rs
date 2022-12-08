use std::io;
use std::io::ErrorKind;
use std::marker::PhantomData;

use crate::{ProtocolCursor, ProtocolError};

impl From<ProtocolError> for io::Error {
    fn from(value: ProtocolError) -> Self {
        io::Error::new(ErrorKind::Other, Box::new(value))
    }
}

#[repr(transparent)]
pub struct StdIOReadProtocolCursor<'a, 'c, C: ProtocolCursor<'c>> {
    cursor: &'a mut C,
    _marker: PhantomData<&'c ()>,
}

impl<'a, 'c, C: ProtocolCursor<'c>> StdIOReadProtocolCursor<'a, 'c, C> {
    pub fn new(cursor: &'a mut C) -> Self {
        Self { cursor, _marker: PhantomData }
    }
}

impl<'a, 'c, C: ProtocolCursor<'c>> io::Read for StdIOReadProtocolCursor<'a, 'c, C> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let length = self.cursor.remaining_bytes().min(buf.len());
        let bytes = self.cursor.take_bytes(length)?;
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.as_mut_ptr(), length);
        }
        Ok(length)
    }
}