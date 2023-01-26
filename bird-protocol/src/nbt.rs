use std::borrow::Cow;
use std::ops::Range;
use crate::{LengthProvidedArray, LengthProvidedBytesArray, ProtocolCursor, ProtocolError, ProtocolReadable, ProtocolResult, ProtocolSize, ProtocolVariantReadable, ProtocolVariantWritable, ProtocolWritable, ProtocolWriter};
use crate::__private::nbt::skip_entered_compound;
use crate::impls::nbt::ProtocolSkipCursor;

#[derive(Debug)]
pub enum BorrowedNbtArray<'a, T> {
    Raw(&'a [u8]),
    Native(&'a [T]),
}

pub type BorrowedI16NbtArray<'a> = BorrowedNbtArray<'a, i16>;
pub type BorrowedI32NbtArray<'a> = BorrowedNbtArray<'a, i32>;
pub type BorrowedI64NbtArray<'a> = BorrowedNbtArray<'a, i64>;
pub type BorrowedF32NbtArray<'a> = BorrowedNbtArray<'a, f32>;
pub type BorrowedF64NbtArray<'a> = BorrowedNbtArray<'a, f64>;

impl<'a, T> Copy for BorrowedNbtArray<'a, T> {}

impl<'a, T> Clone for BorrowedNbtArray<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: ProtocolNbtTag<'a> + Clone> Iterator for BorrowedNbtArray<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Raw(raw) => T::read_nbt(raw).ok(),
            Self::Native(native) => {
                let res = native.get(0)?.clone();
                *native = &native[1..];
                Some(res)
            }
        }
    }
}

impl<'a, T: ProtocolNbtTag<'a> + Clone> BorrowedNbtArray<'a, T> {
    pub fn len(&self) -> anyhow::Result<usize> {
        match self {
            Self::Raw(raw) => {
                let mut counter = 0;
                let mut raw_new_cursor = raw.take_cursor();
                loop {
                    match T::read_nbt(&mut raw_new_cursor) {
                        Ok(_) => counter += 1,
                        Err(ProtocolError::End) => { break; },
                        Err(err) => Err(err)?,
                    }
                }
                Ok(counter)
            },
            Self::Native(native) => Ok(native.len())
        }
    }

    pub fn write_order_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self {
            Self::Raw(raw) => writer.write_bytes(raw),
            Self::Native(native) => for to_write in *native { to_write.write_nbt(writer)? }
        }
        Ok(())
    }

}

pub trait ProtocolNbtTag<'a>: 'a + Sized {
    const NBT_TAG: u8;
    const NBT_SIZE: Range<u32>;

    fn default_nbt() -> Option<Self> {
        None
    }

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize>;

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()>;

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self>;
}


macro_rules! from_default_protocol {
    ($ty: ty, $tag: expr) => {
        impl<'a> ProtocolNbtTag<'a> for $ty {
            const NBT_TAG: u8 = $tag;
            const NBT_SIZE: Range<u32> = {
                assert!(<Self as $crate::ProtocolSize>::SIZE.start == <Self as $crate::ProtocolSize>::SIZE.end);
                <Self as $crate::ProtocolSize>::SIZE
            };

            fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
                let bytes = cursor.take_bytes(Self::NBT_SIZE.start as usize * amount)?;
                Ok(bytes.len())
            }

            fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
                self.write(writer)
            }

            fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
                Self::read(cursor)
            }
        }
    }
}

from_default_protocol!(u8, 1);
from_default_protocol!(i8, 1);
from_default_protocol!(i16, 2);
from_default_protocol!(i32, 3);
from_default_protocol!(i64, 4);
from_default_protocol!(f32, 5);
from_default_protocol!(f64, 6);

impl<'a> ProtocolNbtTag<'a> for &'a [u8] {
    const NBT_TAG: u8 = 7;
    const NBT_SIZE: Range<u32> = (4..4+(i32::MAX as u32));

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
        let mut result = 0;
        for _ in 0..amount {
            let len = i32::read(cursor)? as usize;
            result += len;
            cursor.take_bytes(len)?;
        }
        Ok(result)
    }

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        LengthProvidedBytesArray::<i32, i32>::write_variant(self, writer)
    }

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        LengthProvidedBytesArray::<i32, i32>::read_variant(cursor)
    }
}

impl<'a> ProtocolNbtTag<'a> for Cow<'a, str> {
    const NBT_TAG: u8 = 8;
    const NBT_SIZE: Range<u32> = (2..2+(u16::MAX as u32));

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
        let mut result = 0;
        for _ in 0..amount {
            let len = u16::read(cursor)? as usize;
            result += len;
            cursor.take_bytes(len)?;
        }
        Ok(result)
    }

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        if self.len() > u16::MAX as usize { Err(anyhow::Error::msg("Too big string"))? }
        (self.len() as u16).write(writer)?;
        match cesu8::to_java_cesu8(self) {
            Cow::Owned(owned) => writer.write_bytes(&owned),
            Cow::Borrowed(borrowed) => writer.write_bytes(borrowed)
        };
        Ok(())
    }

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let len = u16::read(cursor)? as usize;
        cesu8::from_java_cesu8(cursor.take_bytes(len)?).map_err(|err| ProtocolError::Any(err.into()))
    }
}

#[derive(Debug)]
pub struct NbtList<'a, T>(pub BorrowedNbtArray<'a, T>);

impl<'a, T> Copy for NbtList<'a, T> {}

impl<'a, T> Clone for NbtList<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: ProtocolNbtTag<'a> + Clone> ProtocolNbtTag<'a> for NbtList<'a, T> {
    const NBT_TAG: u8 = 9;
    const NBT_SIZE: Range<u32> = (5..u32::MAX);

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
        let mut result = 0;
        for _ in 0..amount {
            let tag = u8::read(cursor)?;
            let len = i32::read(cursor)?;
            result += 5;
            if len <= 0 { break; }
            if tag == T::NBT_TAG { return Err(ProtocolError::Any(anyhow::Error::msg("Bad nbt tag"))) }
            result += T::skip_nbt(cursor, len as usize)?;
        }
        Ok(result)
    }

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        T::NBT_TAG.write(writer)?;
        (self.0.len()? as i32).write(writer)?;
        match self.0 {
            BorrowedNbtArray::Raw(raw) => writer.write_bytes(raw),
            BorrowedNbtArray::Native(native) => for to_write in native { to_write.write_nbt(writer)?; }
        };
        Ok(())
    }

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let tag = u8::read(cursor)?;
        let len = i32::read(cursor)?;
        if len <= 0 { return Ok(Self(BorrowedNbtArray::Native(&[]))) };
        if tag != T::NBT_TAG { return Err(ProtocolError::Any(anyhow::Error::msg("Bad nbt tag"))) };
        let mut pcursor = cursor.take_cursor();
        let bytes_len = T::skip_nbt(&mut pcursor, len as usize)?;
        Ok(Self(BorrowedNbtArray::Raw(cursor.take_bytes(bytes_len)?)))
    }
}

macro_rules! borrowed_nbt_array {
    ($ty: ident, $tag: expr, $size_one: expr) => {
        impl<'a> ProtocolNbtTag<'a> for $ty<'a> {
            const NBT_TAG: u8 = $tag;
            const NBT_SIZE: Range<u32> = (4..u32::MAX);

            fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
                let mut result = 0;
                for _ in 0..amount {
                    let len = i32::read(cursor)? as usize;
                    let bytes = cursor.take_bytes(len * $size_one)?;
                    result += $size_one + bytes.len();
                }
                Ok(result)
            }

            fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
                match self {
                    Self::Raw(raw) => (raw.len() / $size_one) as i32,
                    Self::Native(native) => native.len() as i32,
                }.write(writer)?;
                self.write_order_nbt(writer)
            }

            fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
                let len = i32::read(cursor)? as usize;
                Ok(Self::Raw(cursor.take_bytes(len * $size_one)?))
            }
        }
    }
}

borrowed_nbt_array!(BorrowedI32NbtArray, 11, 4);
borrowed_nbt_array!(BorrowedI64NbtArray, 12, 8);

impl<'a, T: ProtocolNbtTag<'a>> ProtocolNbtTag<'a> for Option<T> {
    const NBT_TAG: u8 = T::NBT_TAG;
    const NBT_SIZE: Range<u32> = T::NBT_SIZE;

    fn default_nbt() -> Option<Self> {
        Some(None)
    }

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
        T::skip_nbt(cursor, amount)
    }

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self {
            Some(val) => T::write_nbt(val, writer),
            None => Ok(())
        }
    }

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        T::read_nbt(cursor).map(|val| Some(val))
    }
}

pub const COMPOUND_TAG: u8 = 10;

pub fn enter_compound<'a, C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<()> {
    let tag = u8::read_nbt(cursor)?;
    if tag != COMPOUND_TAG { Err(ProtocolError::Any(anyhow::Error::msg("Expected compound")))? }
    let _ = <Cow<'a, str>>::read_nbt(cursor)?;
    Ok(())
}

pub fn skip_compound<'a, C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<usize> {
    let mut skip_cursor = ProtocolSkipCursor::new(cursor.take_cursor());
    skip_entered_compound(&mut skip_cursor)?;
    cursor.take_bytes(skip_cursor.length)?;
    Ok(skip_cursor.length)
}

pub fn read_compound<'a, C: ProtocolCursor<'a>>(cursor: &mut C, mut func: impl FnMut(u8, &str, &mut C) -> ProtocolResult<()>) -> ProtocolResult<()> {
    loop {
        let tag = u8::read_nbt(cursor)?;
        if tag == 0 { break; }
        let name = <Cow<'a, str>>::read_nbt(cursor)?;
        func(tag, &name, cursor)?;
    }
    Ok(())
}