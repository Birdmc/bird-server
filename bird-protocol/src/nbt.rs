use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::Range;
use crate::{ProtocolCursor, ProtocolError, ProtocolResult, ProtocolWriter};

#[derive(Debug)]
pub enum NbtBorrowedArray<'a, T, const SIZE: usize = 0> {
    Raw(&'a [u8]),
    Native(&'a [T]),
}

pub type NbtBorrowedI16Array<'a> = NbtBorrowedArray<'a, i16, 2>;
pub type NbtBorrowedU16Array<'a> = NbtBorrowedArray<'a, u16, 2>;
pub type NbtBorrowedI32Array<'a> = NbtBorrowedArray<'a, i32, 4>;
pub type NbtBorrowedU32Array<'a> = NbtBorrowedArray<'a, u32, 4>;
pub type NbtBorrowedI64Array<'a> = NbtBorrowedArray<'a, i64, 8>;
pub type NbtBorrowedU64Array<'a> = NbtBorrowedArray<'a, u64, 8>;
pub type NbtBorrowedF32Array<'a> = NbtBorrowedArray<'a, f32, 4>;
pub type NbtBorrowedF64Array<'a> = NbtBorrowedArray<'a, f64, 8>;

impl<'a, T, const SIZE: usize> Copy for NbtBorrowedArray<'a, T, SIZE> {}

impl<'a, T, const SIZE: usize> Clone for NbtBorrowedArray<'a, T, SIZE> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: Clone + NbtTag<'a>, const SIZE: usize> Iterator for NbtBorrowedArray<'a, T, SIZE> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Raw(raw) => T::read_nbt(raw).ok(),
            Self::Native(native) => {
                let (first, rem) = native.split_first()?;
                *native = rem;
                Some(first.clone())
            }
        }
    }
}

impl<'a, T: NbtTag<'a>, const SIZE: usize> NbtBorrowedArray<'a, T, SIZE> {

    fn len(&self) -> ProtocolResult<usize> {
        match self {
            Self::Raw(raw) => match SIZE {
                0 => {
                    let mut cursor = raw.take_cursor();
                    let mut result = 0;
                    while !cursor.is_empty() {
                        T::read_nbt(&mut cursor)?;
                        result += 1;
                    }
                    Ok(result)
                },
                size => {
                    debug_assert!(raw.len() % size == 0);
                    Ok(raw.len() / size)
                }
            },
            Self::Native(native) => Ok(native.len())
        }
    }

    fn write_nbt_values<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self {
            Self::Raw(raw) => writer.write_bytes(raw),
            Self::Native(native) => for to_write in *native { to_write.write_nbt(writer)? },
        }
        Ok(())
    }
    
    fn read_nbt_values<C: ProtocolCursor<'a>>(cursor: &mut C, len: usize) -> ProtocolResult<Self> {
        match SIZE {
            0 => {
                let mut skip_cursor = cursor.take_cursor();
                let raw_len = T::skip_nbt(&mut skip_cursor, len)?;
                Ok(Self::Raw(cursor.take_bytes(raw_len)?))
            },
            size => Ok(Self::Raw(cursor.take_bytes(size * len)?))
        }
    }

    fn skip_nbt_values<C: ProtocolCursor<'a>>(cursor: &mut C, len: usize) -> ProtocolResult<usize> {
        T::skip_nbt(cursor, len)
    }

}

pub const NBT_TAG_END: u8 = 0;
pub const NBT_TAG_BYTE: u8 = 1;
pub const NBT_TAG_SHORT: u8 = 2;
pub const NBT_TAG_INT: u8 = 3;
pub const NBT_TAG_LONG: u8 = 4;
pub const NBT_TAG_FLOAT: u8 = 5;
pub const NBT_TAG_DOUBLE: u8 = 6;
pub const NBT_TAG_BYTE_ARRAY: u8 = 7;
pub const NBT_TAG_STRING: u8 = 8;
pub const NBT_TAG_LIST: u8 = 9;
pub const NBT_TAG_COMPOUND: u8 = 10;
pub const NBT_TAG_INT_ARRAY: u8 = 11;
pub const NBT_TAG_LONG_ARRAY: u8 = 12;

pub trait NbtTagVariant<'a, T> {
    fn get_tag(value: &T) -> anyhow::Result<u8>;

    fn write_nbt_variant<W: ProtocolWriter>(value: &T, writer: &mut W) -> anyhow::Result<()>;

    fn check_tag(tag: u8) -> bool;

    fn read_nbt_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<T>;

    fn skip_nbt_variant<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize>;
}

pub trait NbtTag<'a>: Sized {
    const NBT_TAG: u8;

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()>;

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self>;

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize>;
}

impl<'a, T: NbtTag<'a>> NbtTagVariant<'a, T> for T {
    fn get_tag(_: &T) -> anyhow::Result<u8> {
        Ok(Self::NBT_TAG)
    }

    fn write_nbt_variant<W: ProtocolWriter>(value: &T, writer: &mut W) -> anyhow::Result<()> {
        value.write_nbt(writer)
    }

    fn check_tag(tag: u8) -> bool {
        tag == Self::NBT_TAG
    }

    fn read_nbt_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<T> {
        T::read_nbt(cursor)
    }

    fn skip_nbt_variant<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
        T::skip_nbt(cursor, amount)
    }
}

macro_rules! inherit_from_default_protocol {
    ($($ty: ty = $tag: expr$(,)*)*) => {
        $(
        impl<'a> $crate::nbt::NbtTag<'a> for $ty {
            const NBT_TAG: u8 = $tag;

            fn write_nbt<W: $crate::ProtocolWriter>(&self, writer: &mut W) -> $crate::anyhow::Result<()> {
                <Self as $crate::ProtocolWritable>::write(self, writer)
            }

            fn read_nbt<C: $crate::ProtocolCursor<'a>>(cursor: &mut C) -> $crate::ProtocolResult<Self> {
                <Self as $crate::ProtocolReadable<'a>>::read(cursor)
            }

            fn skip_nbt<C: $crate::ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> $crate::ProtocolResult<usize> {
                let to_skip = (<Self as $crate::ProtocolSize>::SIZE.start as usize) * amount;
                cursor.take_bytes(to_skip)?;
                Ok(to_skip)
            }
        }
        )*
    }
}

inherit_from_default_protocol!(
    bool = NBT_TAG_BYTE,
    i8 = NBT_TAG_BYTE,
    u8 = NBT_TAG_BYTE,
    i16 = NBT_TAG_SHORT,
    u16 = NBT_TAG_SHORT,
    i32 = NBT_TAG_INT,
    u32 = NBT_TAG_INT,
    i64 = NBT_TAG_LONG,
    u64 = NBT_TAG_LONG,
    f32 = NBT_TAG_FLOAT,
    f64 = NBT_TAG_DOUBLE,
);

pub fn write_nbt_str<W: ProtocolWriter>(str: &str, writer: &mut W) -> anyhow::Result<()> {
    (str.len() as u16).write_nbt(writer)?;
    match cesu8::to_java_cesu8(str) {
        Cow::Owned(owned) => writer.write_bytes(&owned),
        Cow::Borrowed(borrowed) => writer.write_bytes(borrowed),
    }
    Ok(())
}

impl<'a> NbtTag<'a> for Cow<'a, str> {
    const NBT_TAG: u8 = NBT_TAG_STRING;

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        write_nbt_str(&self, writer)
    }

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let len = u16::read_nbt(cursor)? as usize;
        cesu8::from_java_cesu8(cursor.take_bytes(len)?).map_err(|err| ProtocolError::Any(err.into()))
    }

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
        let mut result = 0;
        for _ in 0..amount {
            let len = u16::read_nbt(cursor)? as _;
            cursor.take_bytes(len)?;
            result += len + 2;
        }
        Ok(result)
    }
}

impl<'a> NbtTag<'a> for String {
    const NBT_TAG: u8 = NBT_TAG_STRING;

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        write_nbt_str(self.as_str(), writer)
    }

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        Cow::read_nbt(cursor).map(|str: Cow<str>| str.into_owned())
    }

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
        Cow::<str>::skip_nbt(cursor, amount)
    }
}

impl<'a> NbtTag<'a> for &'a [u8] {
    const NBT_TAG: u8 = NBT_TAG_LIST;

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        NBT_TAG_BYTE.write_nbt(writer)?;
        (self.len() as i32).write_nbt(writer)?;
        writer.write_bytes(self);
        Ok(())
    }

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let tag = u8::read_nbt(cursor)?;
        let len = i32::read_nbt(cursor)?;
        if len <= 0 { return Ok(&[]) }
        if tag != NBT_TAG_BYTE { return Err(ProtocolError::Any(anyhow::Error::msg("Bad nbt tag"))) }
        cursor.take_bytes(len as _)
    }

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
        let mut result = 0;
        for _ in 0..amount {
            let tag = u8::read_nbt(cursor)?;
            let len = i32::read_nbt(cursor)?;
            if len <= 0 { result += 5; continue; }
            if tag != NBT_TAG_BYTE { return Err(ProtocolError::Any(anyhow::Error::msg("Bad nbt tag"))) }
            cursor.take_bytes(len as _)?;
            result += 5 + len as usize;
        }
        Ok(result)
    }
}

impl<'a, T: NbtTag<'a> + 'a, const SIZE: usize> NbtTag<'a> for NbtBorrowedArray<'a, T, SIZE> {
    const NBT_TAG: u8 = NBT_TAG_LIST;

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        T::NBT_TAG.write_nbt(writer)?;
        (self.len()? as i32).write_nbt(writer)?;
        self.write_nbt(writer)
    }

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let tag = u8::read_nbt(cursor)?;
        let len = i32::read_nbt(cursor)?;
        if len <= 0 { return Ok(NbtBorrowedArray::Native(&[])) }
        if tag != T::NBT_TAG { return Err(ProtocolError::Any(anyhow::Error::msg("Bad nbt tag"))) }
        Self::read_nbt_values(cursor, len as _)
    }

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
        let mut result = 0;
        for _ in 0..amount {
            let tag = u8::read_nbt(cursor)?;
            let len = i32::read_nbt(cursor)?;
            if len <= 0 { result += 5; continue; }
            if tag != T::NBT_TAG { return Err(ProtocolError::Any(anyhow::Error::msg("Bad nbt tag"))) }
            result += 5 + Self::skip_nbt_values(cursor, len as _)?;
        }
        Ok(result)
    }
}

impl<'a, T: NbtTag<'a>> NbtTag<'a> for Vec<T> {
    const NBT_TAG: u8 = NBT_TAG_LIST;

    fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        T::NBT_TAG.write_nbt(writer)?;
        (self.len() as i32).write_nbt(writer)?;
        for tag in self { tag.write_nbt(writer)? }
        Ok(())
    }

    fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let tag = u8::read_nbt(cursor)?;
        let len = i32::read_nbt(cursor)?;
        if len <= 0 { return Ok(Vec::new()) }
        if tag != T::NBT_TAG { return Err(ProtocolError::Any(anyhow::Error::msg("Bad nbt tag"))) }
        let mut result = Vec::new();
        for _ in 0..len {
            result.push(T::read_nbt(cursor)?);
        }
        Ok(result)
    }

    fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize> {
        let mut result = 0;
        for _ in 0..amount {
            let tag = u8::read_nbt(cursor)?;
            let len = i32::read_nbt(cursor)?;
            if len <= 0 { result += 5; continue; }
            if tag != T::NBT_TAG { return Err(ProtocolError::Any(anyhow::Error::msg("Bad nbt tag"))) }
            result += 5 + T::skip_nbt(cursor, len as _)?;
        }
        Ok(result)
    }
}

pub struct NbtByteArray;
pub struct NbtIntArray;
pub struct NbtLongArray;

macro_rules! inherit_from_nbt_tag {
    ($($inheritance: ident => $ty: ty = $tag: expr$(,)*)*) => {
        $(
        impl<'a> $crate::nbt::NbtTagVariant<'a, $inheritance<'a>> for $ty {
            fn get_tag(_: &$inheritance<'a>) -> $crate::anyhow::Result<u8> {
                Ok($tag)
            }

            fn write_nbt_variant<W: $crate::ProtocolWriter>(value: &$inheritance<'a>, writer: &mut W) -> $crate::anyhow::Result<()> {
                <$inheritance::<'a> as $crate::nbt::NbtTag<'a>>::write_nbt(value, writer)
            }

            fn check_tag(tag: u8) -> bool {
                tag == $tag
            }

            fn read_nbt_variant<C: $crate::ProtocolCursor<'a>>(cursor: &mut C) -> $crate::ProtocolResult<$inheritance<'a>> {
                <$inheritance::<'a> as $crate::nbt::NbtTag<'a>>::read_nbt(cursor)
            }

            fn skip_nbt_variant<C: $crate::ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> $crate::ProtocolResult<usize> {
                <$inheritance::<'a> as $crate::nbt::NbtTag<'a>>::skip_nbt(cursor, amount)
            }
        }
        )*
    }
}

type U8Array<'a> = &'a [u8];

inherit_from_nbt_tag!(
    U8Array => NbtByteArray = NBT_TAG_BYTE_ARRAY,
    NbtBorrowedI32Array => NbtIntArray = NBT_TAG_INT_ARRAY,
    NbtBorrowedI64Array => NbtLongArray = NBT_TAG_LONG_ARRAY,
);

pub mod compound {
    use super::*;

    pub fn read_nbt_compound<'a, C: ProtocolCursor<'a>>(
        cursor: &mut C,
        mut fun: impl FnMut(u8, Cow<'a, str>, &mut C) -> ProtocolResult<()>
    ) -> ProtocolResult<()> {
        loop {
            let tag = u8::read_nbt(cursor)?;
            if tag == NBT_TAG_END { break; }
            let name = Cow::read_nbt(cursor)?;
            fun(tag, name, cursor)?;
        }
        Ok(())
    }

}