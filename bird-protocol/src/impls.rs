use std::{borrow::Cow, str::from_utf8};

use serde::{Deserialize, Serialize};

use crate::*;

pub const fn add_u32_without_overflow(first: u32, second: u32) -> u32 {
    match u32::MAX - first < second {
        true => u32::MAX,
        false => first + second,
    }
}

pub const fn max_u32(first: u32, second: u32) -> u32 {
    match first > second {
        true => first,
        false => second,
    }
}

pub const fn min_u32(first: u32, second: u32) -> u32 {
    match first < second {
        true => first,
        false => second,
    }
}

impl<'a, T: ProtocolReadable<'a>> ProtocolVariantReadable<'a, T> for T {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<T> {
        T::read(cursor)
    }
}

impl<T: ProtocolWritable> ProtocolVariantWritable<T> for T {
    fn write_variant<W: ProtocolWriter>(object: &T, writer: &mut W) -> anyhow::Result<()> {
        object.write(writer)
    }
}

macro_rules! delegate_size {
    ($($ty: ty = $delegate: ty$(,)*)*) => {
        $(
            impl ProtocolSize for $ty {
                const SIZE: Range<u32> = <$delegate as ProtocolSize>::SIZE;
            }
        )*
    }
}

macro_rules! fixed_range_size {
    ($($ty: ty = ($min: expr, $max: expr)$(,)*)*) => {
        $(
            impl ProtocolSize for $ty {
                const SIZE: Range<u32> = ($min..$max);
            }
        )*
    }
}

macro_rules! fixed_size {
    ($($ty: ty = $value: expr$(,)*)*) => {
        $(
            fixed_range_size!($ty = ($value, $value));
        )*
    }
}

macro_rules! number_impl {
    ($ty: ty) => {
        fixed_size!($ty = std::mem::size_of::<$ty>() as u32);

        impl<'a> ProtocolReadable<'a> for $ty {
            fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
                let mut bytes = [0u8; std::mem::size_of::<Self>()];
                let slice = cursor.take_bytes(bytes.len())?;
                unsafe {
                    // Safety. Slice reference is valid, bytes reference also. They don't overlap
                    std::ptr::copy_nonoverlapping(slice.as_ptr(), bytes.as_mut_ptr(), bytes.len())
                }
                Ok(Self::from_be_bytes(bytes))
            }
        }

        impl ProtocolWritable for $ty {
            fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
                Ok(writer.write_fixed_bytes(self.to_be_bytes()))
            }
        }
    };
    ($($ty: ty$(,)*)*) => {
        $(number_impl!($ty);)*
    }
}

number_impl!(i8 u8 i16 u16 i32 u32 i64 u64 i128 u128 f32 f64);
fixed_size!(bool = 1);
fixed_range_size!(VarInt = (1, 5), VarLong = (1, 10));

impl<'a> ProtocolReadable<'a> for bool {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        u8::read(cursor).map(|val| val != 0)
    }
}

impl ProtocolWritable for bool {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self {
            true => 1u8,
            false => 0u8,
        }
            .write(writer)
    }
}

macro_rules! var_number_impl {
    ($($ty: ty = ($signed: ty, $unsigned: ty)$(,)*)*) => {
        $(
            impl<'a> ProtocolVariantReadable<'a, $signed> for $ty {
                fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<$signed> {
                    let mut value: $signed = 0;
                    let mut position = 0u8;
                    loop {
                        let current_byte = cursor.take_byte()?;
                        value |= ((current_byte & 0x7F) << position) as $signed;
                        if ((current_byte & 0x80) == 0) {
                            break;
                        }
                        position += 7;
                        if (position >= (std::mem::size_of::<$signed>() * 8) as u8) {
                            return Err(anyhow::Error::msg("Var number is too big").into());
                        }
                    }
                    Ok(value)
                }
            }

            impl ProtocolVariantWritable<$signed> for $ty {
                fn write_variant<W: ProtocolWriter>(object: &$signed, writer: &mut W) -> anyhow::Result<()> {
                    let mut object = *object as $unsigned;
                    loop {
                        if ((object & !0x7F) == 0) {
                            writer.write_byte(object as u8);
                            break;
                        }
                        writer.write_byte((object as u8 & 0x7F) | 0x80);
                        object >>= 7;
                    }
                    Ok(())
                }
            }
        )*
    }
}

var_number_impl!(VarInt = (i32, u32), VarLong = (i64, u64));

impl<T: ProtocolSize> ProtocolSize for Option<T> {
    const SIZE: Range<u32> = (1..add_u32_without_overflow(T::SIZE.end, 1));
}

impl<T: ProtocolWritable> ProtocolWritable for Option<T> {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self {
            Some(object) => {
                true.write(writer)?;
                object.write(writer)
            }
            None => false.write(writer),
        }
    }
}

impl<'a, T: ProtocolReadable<'a>> ProtocolReadable<'a> for Option<T> {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        Ok(match bool::read(cursor)? {
            true => Some(T::read(cursor)?),
            false => None,
        })
    }
}

pub fn write_bytes_with_limit<W: ProtocolWriter, const LIMIT: usize>(
    object: &[u8],
    writer: &mut W,
) -> anyhow::Result<()> {
    match object.len() <= LIMIT {
        true => {
            VarInt::write_variant(&(object.len() as i32), writer)?;
            Ok(writer.write_bytes(object))
        }
        false => Err(anyhow::Error::msg("Too long bytes")),
    }
}

pub fn write_str_with_limit<W: ProtocolWriter, const LIMIT: usize>(
    object: &str,
    writer: &mut W,
) -> anyhow::Result<()> {
    match object.len() <= LIMIT {
        true => {
            VarInt::write_variant(&(object.as_bytes().len() as i32), writer)?;
            Ok(writer.write_bytes(object.as_bytes()))
        }
        false => Err(anyhow::Error::msg("Too long string")),
    }
}

pub fn read_str_with_limit<'a, C: ProtocolCursor<'a>, const LIMIT: usize>(
    cursor: &mut C,
) -> ProtocolResult<&'a str> {
    let length = VarInt::read_variant(cursor)? as usize;
    match length <= LIMIT {
        true => from_utf8(cursor.take_bytes(length)?).map_err(|err| ProtocolError::Any(err.into())),
        false => Err(anyhow::Error::msg("Too long string").into()),
    }
}

pub const DEFAULT_LIMIT: usize = 32767;
pub const CHAT_LIMIT: usize = 262144;

fixed_range_size!(&str = (VarInt::SIZE.start, DEFAULT_LIMIT as u32 * 4 + 3));

impl ProtocolWritable for &str {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        write_str_with_limit::<W, DEFAULT_LIMIT>(self, writer)
    }
}

impl<'a> ProtocolReadable<'a> for &'a str {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        read_str_with_limit::<C, DEFAULT_LIMIT>(cursor)
    }
}

delegate_size!(String = &str, Cow<'_, str> = &str);

impl ProtocolWritable for String {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        self.as_str().write(writer)
    }
}

impl<'a> ProtocolReadable<'a> for String {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        <&'a str>::read(cursor).map(|value| value.into())
    }
}

impl<'a> ProtocolWritable for Cow<'a, str> {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self {
            Cow::Owned(owned) => owned.write(writer),
            Cow::Borrowed(borrowed) => borrowed.write(writer),
        }
    }
}

impl<'a> ProtocolReadable<'a> for Cow<'a, str> {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        <&'a str>::read(cursor).map(|value| Cow::Borrowed(value))
    }
}

impl ProtocolSize for RemainingBytesArray {
    const SIZE: Range<u32> = (0..u32::MAX);
}

impl ProtocolVariantWritable<[u8]> for RemainingBytesArray {
    fn write_variant<W: ProtocolWriter>(object: &[u8], writer: &mut W) -> anyhow::Result<()> {
        Ok(writer.write_bytes(object))
    }
}

impl ProtocolVariantWritable<Vec<u8>> for RemainingBytesArray {
    fn write_variant<W: ProtocolWriter>(object: &Vec<u8>, writer: &mut W) -> anyhow::Result<()> {
        Self::write_variant(object.as_slice(), writer)
    }
}

impl ProtocolVariantWritable<Cow<'_, [u8]>> for RemainingBytesArray {
    fn write_variant<W: ProtocolWriter>(
        object: &Cow<'_, [u8]>,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        Self::write_variant(
            match object {
                Cow::Owned(owned) => owned.as_slice(),
                Cow::Borrowed(borrowed) => borrowed,
            },
            writer,
        )
    }
}

impl<'a> ProtocolVariantReadable<'a, &'a [u8]> for RemainingBytesArray {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<&'a [u8]> {
        cursor.take_bytes(cursor.remaining_bytes())
    }
}

impl<'a> ProtocolVariantReadable<'a, Vec<u8>> for RemainingBytesArray {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Vec<u8>> {
        Self::read_variant(cursor).map(|slice: &'a [u8]| slice.into())
    }
}

impl<'a> ProtocolVariantReadable<'a, Cow<'a, [u8]>> for RemainingBytesArray {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Cow<'a, [u8]>> {
        Self::read_variant(cursor).map(|slice| Cow::Borrowed(slice))
    }
}

impl<V, VV: ProtocolSize> ProtocolSize for RemainingArray<V, VV> {
    const SIZE: Range<u32> = (0..u32::MAX);
}

impl<V, VV: ProtocolVariantWritable<V>> ProtocolVariantWritable<[V]> for RemainingArray<V, VV> {
    fn write_variant<W: ProtocolWriter>(object: &[V], writer: &mut W) -> anyhow::Result<()> {
        for value in object {
            VV::write_variant(value, writer)?
        }
        Ok(())
    }
}

impl<V, VV: ProtocolVariantWritable<V>> ProtocolVariantWritable<Vec<V>> for RemainingArray<V, VV> {
    fn write_variant<W: ProtocolWriter>(object: &Vec<V>, writer: &mut W) -> anyhow::Result<()> {
        Self::write_variant(object.as_slice(), writer)
    }
}

impl<V: Clone, VV: ProtocolVariantWritable<V>> ProtocolVariantWritable<Cow<'_, [V]>>
for RemainingArray<V, VV>
{
    fn write_variant<W: ProtocolWriter>(
        object: &Cow<'_, [V]>,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        Self::write_variant(
            match object {
                Cow::Owned(owned) => owned.as_slice(),
                Cow::Borrowed(borrowed) => *borrowed,
            },
            writer,
        )
    }
}

impl<'a, V: 'a, VV: ProtocolVariantReadable<'a, V>> ProtocolVariantReadable<'a, Vec<V>>
for RemainingArray<V, VV>
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Vec<V>> {
        let mut result = Vec::new();
        while cursor.remaining_bytes() != 0 {
            result.push(VV::read_variant(cursor)?);
        }
        Ok(result)
    }
}

impl<'a, V: 'a + Clone, VV: ProtocolVariantReadable<'a, V>>
ProtocolVariantReadable<'a, Cow<'static, [V]>> for RemainingArray<V, VV>
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Cow<'static, [V]>> {
        Self::read_variant(cursor).map(|vec| Cow::Owned(vec))
    }
}

macro_rules! primitive_length {
    ($($ty: ty$(,)*)*) => {
        $(
            impl ProtocolLength for $ty {
                fn into_usize(self) -> usize {
                    self as usize
                }

                fn from_usize(size: usize) -> Self {
                    size as Self
                }
            }
        )*
    }
}

primitive_length!(i8 u8 i16 u16 i32 u32 i64 u64 i128 u128);

impl<L, LV: ProtocolSize> ProtocolSize for LengthProvidedBytesArray<L, LV> {
    const SIZE: Range<u32> = (LV::SIZE.start..u32::MAX);
}

impl<L: ProtocolLength, LV: ProtocolVariantWritable<L>> ProtocolVariantWritable<[u8]>
for LengthProvidedBytesArray<L, LV>
{
    fn write_variant<W: ProtocolWriter>(object: &[u8], writer: &mut W) -> anyhow::Result<()> {
        LV::write_variant(&L::from_usize(object.len()), writer)?;
        Ok(writer.write_bytes(object))
    }
}

impl<L: ProtocolLength, LV: ProtocolVariantWritable<L>> ProtocolVariantWritable<Vec<u8>>
for LengthProvidedBytesArray<L, LV>
{
    fn write_variant<W: ProtocolWriter>(object: &Vec<u8>, writer: &mut W) -> anyhow::Result<()> {
        Self::write_variant(object.as_slice(), writer)
    }
}

impl<L: ProtocolLength, LV: ProtocolVariantWritable<L>> ProtocolVariantWritable<Cow<'_, [u8]>>
for LengthProvidedBytesArray<L, LV>
{
    fn write_variant<W: ProtocolWriter>(
        object: &Cow<'_, [u8]>,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        Self::write_variant(
            match object {
                Cow::Owned(owned) => owned.as_slice(),
                Cow::Borrowed(borrowed) => borrowed,
            },
            writer,
        )
    }
}

impl<'a, L: ProtocolLength, LV: ProtocolVariantReadable<'a, L>>
ProtocolVariantReadable<'a, &'a [u8]> for LengthProvidedBytesArray<L, LV>
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<&'a [u8]> {
        let length = L::into_usize(LV::read_variant(cursor)?);
        cursor.take_bytes(length)
    }
}

impl<'a, L: ProtocolLength, LV: ProtocolVariantReadable<'a, L>> ProtocolVariantReadable<'a, Vec<u8>>
for LengthProvidedBytesArray<L, LV>
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Vec<u8>> {
        Self::read_variant(cursor).map(|slice: &'a [u8]| slice.into())
    }
}

impl<'a, L: ProtocolLength, LV: ProtocolVariantReadable<'a, L>>
ProtocolVariantReadable<'a, Cow<'a, [u8]>> for LengthProvidedBytesArray<L, LV>
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Cow<'a, [u8]>> {
        Self::read_variant(cursor).map(|slice| Cow::Borrowed(slice))
    }
}

impl<L, LV: ProtocolSize, V, VV> ProtocolSize for LengthProvidedArray<L, LV, V, VV> {
    const SIZE: Range<u32> = (LV::SIZE.start..u32::MAX);
}

impl<L: ProtocolLength, LV: ProtocolVariantWritable<L>, V, VV: ProtocolVariantWritable<V>>
ProtocolVariantWritable<[V]> for LengthProvidedArray<L, LV, V, VV>
{
    fn write_variant<W: ProtocolWriter>(object: &[V], writer: &mut W) -> anyhow::Result<()> {
        LV::write_variant(&L::from_usize(object.len()), writer)?;
        for value in object {
            VV::write_variant(value, writer)?;
        }
        Ok(())
    }
}

impl<L: ProtocolLength, LV: ProtocolVariantWritable<L>, V, VV: ProtocolVariantWritable<V>>
ProtocolVariantWritable<Vec<V>> for LengthProvidedArray<L, LV, V, VV>
{
    fn write_variant<W: ProtocolWriter>(object: &Vec<V>, writer: &mut W) -> anyhow::Result<()> {
        Self::write_variant(object.as_slice(), writer)
    }
}

impl<
    L: ProtocolLength,
    LV: ProtocolVariantWritable<L>,
    V: Clone,
    VV: ProtocolVariantWritable<V>,
> ProtocolVariantWritable<Cow<'_, [V]>> for LengthProvidedArray<L, LV, V, VV>
{
    fn write_variant<W: ProtocolWriter>(
        object: &Cow<'_, [V]>,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        Self::write_variant(
            match object {
                Cow::Owned(owned) => owned.as_slice(),
                Cow::Borrowed(borrowed) => borrowed,
            },
            writer,
        )
    }
}

impl<
    'a,
    L: ProtocolLength,
    LV: ProtocolVariantReadable<'a, L>,
    V: 'a,
    VV: ProtocolVariantReadable<'a, V>,
> ProtocolVariantReadable<'a, Vec<V>> for LengthProvidedArray<L, LV, V, VV>
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Vec<V>> {
        let length = L::into_usize(LV::read_variant(cursor)?);
        let mut result = Vec::new();
        for _ in 0..length {
            result.push(VV::read_variant(cursor)?);
        }
        Ok(result)
    }
}

impl<
    'a,
    L: ProtocolLength,
    LV: ProtocolVariantReadable<'a, L>,
    V: 'a + Clone,
    VV: ProtocolVariantReadable<'a, V>,
> ProtocolVariantReadable<'a, Cow<'a, [V]>> for LengthProvidedArray<L, LV, V, VV>
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Cow<'a, [V]>> {
        Self::read_variant(cursor).map(|vec| Cow::Owned(vec))
    }
}

fixed_range_size!(Json = (VarInt::SIZE.start, (CHAT_LIMIT as u32 * 4 + 3)));

impl<T: Serialize> ProtocolVariantWritable<T> for Json {
    fn write_variant<W: ProtocolWriter>(object: &T, writer: &mut W) -> anyhow::Result<()> {
        write_str_with_limit::<W, CHAT_LIMIT>(serde_json::to_string(object)?.as_str(), writer)
    }
}

impl<'a, T: Deserialize<'a>> ProtocolVariantReadable<'a, T> for Json {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<T> {
        serde_json::from_str(read_str_with_limit::<C, CHAT_LIMIT>(cursor)?)
            .map_err(|err| ProtocolError::Any(err.into()))
    }
}
