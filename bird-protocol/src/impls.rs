use std::{borrow::Cow, str::from_utf8};
use std::mem::{MaybeUninit, size_of};
use euclid::{Vector2D, Vector3D};
use bird_chat::component::Component;
use bird_chat::identifier::{Identifier, IdentifierInner};
use bird_util::*;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::*;

macro_rules! protocol_raw {
    ($($ty: ty$(,)+)*) => {
        $(unsafe impl ProtocolRaw for $ty {})*
    }
}

protocol_raw!(u8, i8, bool, Uuid,);

#[cfg(target_endian = "big")]
protocol_raw!(u16, i16, u32, i32, u64, i64, u128, i128);

unsafe impl<U> ProtocolRaw for Vector3D<u8, U> {}

unsafe impl<U> ProtocolRaw for Vector3D<i8, U> {}

unsafe impl<U> ProtocolRaw for Vector2D<u8, U> {}

unsafe impl<U> ProtocolRaw for Vector2D<i8, U> {}

macro_rules! gen_u32_operation {
    ($ty: ty, $new_name: ident, $func: ident, $default_value: expr) => {
        pub const fn $new_name<const N: usize>(array: [$ty; N]) -> $ty {
            if array.len() == 0 { return $default_value; }
            let mut counter = 1;
            let mut value = array[0];
            while counter < array.len() {
                value = $func(value, array[counter]);
                counter += 1;
            }
            value
        }
    }
}

gen_u32_operation!(u32, add_u32_without_overflow_array, add_u32_without_overflow, 0);
gen_u32_operation!(u32, max_u32_array, max_u32, u32::MAX);
gen_u32_operation!(u32, min_u32_array, min_u32, 0);
gen_u32_operation!((u32, u32), add_protocol_sizes, add_u32_range_without_overflow, (0, 0));

#[macro_export]
macro_rules! add_protocol_sizes_ty {
    ($($ty: ty$(,)*)*) => {
        {
            let o = $crate::add_protocol_sizes([$((<$ty as $crate::ProtocolSize>::SIZE.start, <$ty as $crate::ProtocolSize>::SIZE.end),)*]);
            (o.0..o.1)
        }
    }
}

pub const fn size_of_val<T: ProtocolSize>(_: &T) -> Range<u32> {
    T::SIZE
}

#[inline]
pub fn read_of_val<'a, T: ProtocolReadable<'a>, C: ProtocolCursor<'a>>(_: &T, cursor: &mut C) -> ProtocolResult<T> {
    T::read(cursor)
}

#[inline]
pub fn read_of_variant_val<'a, T, V: ProtocolVariantReadable<'a, T>, C: ProtocolCursor<'a>>(_: &T, cursor: &mut C) -> ProtocolResult<T> {
    V::read_variant(cursor)
}

pub const fn add_u32_range_without_overflow(first: (u32, u32), second: (u32, u32)) -> (u32, u32) {
    (add_u32_without_overflow(first.0, second.0), add_u32_without_overflow(first.1, second.1))
}

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

macro_rules! var_number_lower_nums_impl {
    ($($ty: ty => $orig: ty = ($($lower_ty: ty$(,)*)*)$(,)*)*) => {
        $($(
            impl<'a> ProtocolVariantReadable<'a, $lower_ty> for $ty {
                fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<$lower_ty> {
                    let v: $orig = Self::read_variant(cursor)?;
                    Ok(v as $lower_ty)
                }
            }

            impl ProtocolVariantWritable<$lower_ty> for $ty {
                fn write_variant<W: ProtocolWriter>(object: &$lower_ty, writer: &mut W) -> anyhow::Result<()> {
                    Self::write_variant(&(*object as $orig), writer)
                }
            }
        )*)*
    }
}

var_number_lower_nums_impl!(
    VarInt => i32 = (i8, u8, i16, u16, u32),
    VarLong => i64 = (i8, u8, i16, u16, i32, u32, u64)
);

impl<'a> ProtocolVariantReadable<'a, bool> for VarInt {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<bool> {
        bool::read(cursor)
    }
}

impl<'a> ProtocolVariantWritable<bool> for VarInt {
    fn write_variant<W: ProtocolWriter>(object: &bool, writer: &mut W) -> anyhow::Result<()> {
        object.write(writer)
    }
}

impl<'a> ProtocolVariantReadable<'a, bool> for VarLong {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<bool> {
        bool::read(cursor)
    }
}

impl<'a> ProtocolVariantWritable<bool> for VarLong {
    fn write_variant<W: ProtocolWriter>(object: &bool, writer: &mut W) -> anyhow::Result<()> {
        object.write(writer)
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

#[inline]
fn too_long_string() -> anyhow::Error {
    anyhow::Error::msg("Too long string")
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
        false => Err(too_long_string()),
    }
}

pub fn read_str_with_limit<'a, C: ProtocolCursor<'a>, const LIMIT: usize>(
    cursor: &mut C,
) -> ProtocolResult<&'a str> {
    let length: i32 = VarInt::read_variant(cursor)?;
    let length = length as usize;
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

const fn byte_array_into_t_array<T: Sized>(array: &[u8]) -> &[T] {
    unsafe { std::slice::from_raw_parts(array.as_ptr() as *const T, array.len() / std::mem::size_of::<T>()) }
}

const fn t_array_into_byte_array<T: Sized>(array: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(array.as_ptr() as *const u8, array.len() * std::mem::size_of::<T>()) }
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

impl<'a, L: ProtocolLength, LV: ProtocolVariantReadable<'a, L>> ProtocolVariantReadable<'a, usize> for ProtocolLengthProvidedDeterminer<L, LV> {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<usize> {
        LV::read_variant(cursor).map(|l| l.into_usize())
    }
}

impl<L, LV: ProtocolSize> ProtocolSize for ProtocolLengthProvidedDeterminer<L, LV> { const SIZE: Range<u32> = LV::SIZE; }

impl<L: ProtocolLength, LV: ProtocolVariantWritable<L>> ProtocolVariantWritable<usize> for ProtocolLengthProvidedDeterminer<L, LV> {
    fn write_variant<W: ProtocolWriter>(object: &usize, writer: &mut W) -> anyhow::Result<()> {
        LV::write_variant(&L::from_usize(*object), writer)
    }
}

impl<'a, L: ProtocolLength, LV: ProtocolVariantReadable<'a, L> + ProtocolVariantWritable<L> + ProtocolSize> ProtocolLengthDeterminer<'a> for ProtocolLengthProvidedDeterminer<L, LV> {
    const ELEMENT_COUNT: bool = true;
}

impl<'a> ProtocolVariantReadable<'a, usize> for ProtocolLengthRemainingDeterminer {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<usize> {
        Ok(cursor.remaining_bytes())
    }
}

impl ProtocolSize for ProtocolLengthRemainingDeterminer { const SIZE: Range<u32> = (0..0); }

impl ProtocolVariantWritable<usize> for ProtocolLengthRemainingDeterminer {
    fn write_variant<W: ProtocolWriter>(_object: &usize, _writer: &mut W) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<'a> ProtocolLengthDeterminer<'a> for ProtocolLengthRemainingDeterminer {
    const ELEMENT_COUNT: bool = false;
}

impl<'a, const N: usize> ProtocolVariantReadable<'a, usize> for ProtocolLengthConstDeterminer<N> {
    fn read_variant<C: ProtocolCursor<'a>>(_cursor: &mut C) -> ProtocolResult<usize> {
        Ok(N)
    }
}

impl<const N: usize> ProtocolSize for ProtocolLengthConstDeterminer<N> { const SIZE: Range<u32> = (0..0); }

impl<const N: usize> ProtocolVariantWritable<usize> for ProtocolLengthConstDeterminer<N> {
    fn write_variant<W: ProtocolWriter>(_object: &usize, _writer: &mut W) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<'a, const N: usize> ProtocolLengthDeterminer<'a> for ProtocolLengthConstDeterminer<N> {
    const ELEMENT_COUNT: bool = true;
}

impl<'a, V, VV, T: ProtocolLengthDeterminer<'a>> ProtocolSize for LengthFunctionRawArray<V, VV, T> {
    const SIZE: Range<u32> = (T::SIZE.start..u32::MAX);
}

impl<'a, V: Sized, VV: ProtocolRaw, T: ProtocolLengthDeterminer<'a>> ProtocolVariantWritable<[V]>
for LengthFunctionRawArray<V, VV, T>
{
    fn write_variant<W: ProtocolWriter>(object: &[V], writer: &mut W) -> anyhow::Result<()> {
        T::write_variant(&(object.len() * if T::ELEMENT_COUNT { 1 } else { std::mem::size_of::<V>() }), writer)?;
        Ok(writer.write_bytes(t_array_into_byte_array(object)))
    }
}

impl<'a, V: Sized, VV: ProtocolRaw, T: ProtocolLengthDeterminer<'a>> ProtocolVariantWritable<&'a [V]> for LengthFunctionRawArray<V, VV, T> {
    fn write_variant<W: ProtocolWriter>(object: &&'a [V], writer: &mut W) -> anyhow::Result<()> {
        Self::write_variant(*object, writer)
    }
}

impl<'a, V: Sized, VV: ProtocolRaw, T: ProtocolLengthDeterminer<'a>> ProtocolVariantWritable<Vec<V>> for LengthFunctionRawArray<V, VV, T> {
    fn write_variant<W: ProtocolWriter>(object: &Vec<V>, writer: &mut W) -> anyhow::Result<()> {
        Self::write_variant(object.as_slice(), writer)
    }
}

impl<'a, V: Sized + Clone, VV: ProtocolRaw, T: ProtocolLengthDeterminer<'a>> ProtocolVariantWritable<Cow<'_, [V]>> for LengthFunctionRawArray<V, VV, T>
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

impl<'a, V: Sized, VV: ProtocolRaw, T: ProtocolLengthDeterminer<'a>> ProtocolVariantReadable<'a, &'a [V]> for LengthFunctionRawArray<V, VV, T>
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<&'a [V]> {
        let length = T::read_variant(cursor)? / if T::ELEMENT_COUNT { 1 } else { std::mem::size_of::<V>() };
        Ok(byte_array_into_t_array(cursor.take_bytes(length)?))
    }
}

impl<'a, V: Sized + Clone + 'a, VV: ProtocolRaw, T: ProtocolLengthDeterminer<'a>> ProtocolVariantReadable<'a, Vec<V>> for LengthFunctionRawArray<V, VV, T>
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Vec<V>> {
        Self::read_variant(cursor).map(|slice: &'a [V]| slice.into())
    }
}

impl<'a, V: Sized, VV: ProtocolRaw, T: ProtocolLengthDeterminer<'a>> ProtocolVariantReadable<'a, Cow<'a, [V]>> for LengthFunctionRawArray<V, VV, T>
    where [V]: ToOwned
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Cow<'a, [V]>> {
        Self::read_variant(cursor).map(|slice| Cow::Borrowed(slice))
    }
}

impl<'a, V, VV, T: ProtocolLengthDeterminer<'a>> ProtocolSize for LengthFunctionArray<V, VV, T> {
    const SIZE: Range<u32> = (T::SIZE.start..u32::MAX);
}

impl<'a, V, VV: ProtocolVariantWritable<V>, T: ProtocolLengthDeterminer<'a>> ProtocolVariantWritable<[V]> for LengthFunctionArray<V, VV, T>
    where ConstAssert<{ T::ELEMENT_COUNT }>: ConstAssertTrue
{
    fn write_variant<W: ProtocolWriter>(object: &[V], writer: &mut W) -> anyhow::Result<()> {
        T::write_variant(&object.len(), writer)?;
        for value in object {
            VV::write_variant(value, writer)?;
        }
        Ok(())
    }
}

impl<'a, V, VV: ProtocolVariantWritable<V>, T: ProtocolLengthDeterminer<'a>> ProtocolVariantWritable<Vec<V>> for LengthFunctionArray<V, VV, T>
    where ConstAssert<{ T::ELEMENT_COUNT }>: ConstAssertTrue
{
    fn write_variant<W: ProtocolWriter>(object: &Vec<V>, writer: &mut W) -> anyhow::Result<()> {
        Self::write_variant(object.as_slice(), writer)
    }
}

impl<
    'a,
    V: Clone,
    VV: ProtocolVariantWritable<V>,
    T: ProtocolLengthDeterminer<'a>
> ProtocolVariantWritable<Cow<'_, [V]>> for LengthFunctionArray<V, VV, T>
    where ConstAssert<{ T::ELEMENT_COUNT }>: ConstAssertTrue
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
    V: 'a,
    VV: ProtocolVariantReadable<'a, V>,
    T: ProtocolLengthDeterminer<'a>
> ProtocolVariantReadable<'a, Vec<V>> for LengthFunctionArray<V, VV, T>
    where ConstAssert<{ T::ELEMENT_COUNT }>: ConstAssertTrue
{
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Vec<V>> {
        let length = T::read_variant(cursor)?;
        let mut result = Vec::new();
        for _ in 0..length {
            result.push(VV::read_variant(cursor)?);
        }
        Ok(result)
    }
}

impl<
    'a,
    V: 'a + Clone,
    VV: ProtocolVariantReadable<'a, V>,
    T: ProtocolLengthDeterminer<'a>
> ProtocolVariantReadable<'a, Cow<'a, [V]>> for LengthFunctionArray<V, VV, T>
    where ConstAssert<{ T::ELEMENT_COUNT }>: ConstAssertTrue
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

fixed_size!(Uuid = 16);

impl ProtocolWritable for Uuid {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        Ok(writer.write_bytes(self.as_bytes().as_slice()))
    }
}

impl<'a> ProtocolReadable<'a> for Uuid {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let mut bytes = [0u8; 16];
        let took = cursor.take_bytes(16)?;
        unsafe {
            std::ptr::copy_nonoverlapping(took.as_ptr(), bytes.as_mut_ptr(), 16);
        }
        Ok(Uuid::from_bytes(bytes))
    }
}

fixed_range_size!(Component<'_> = (1, (262144 * 4) + 3));

impl<'a> ProtocolWritable for Component<'a> {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        write_str_with_limit::<_, CHAT_LIMIT>(serde_json::to_string(self)?.as_str(), writer)
    }
}

impl<'a> ProtocolReadable<'a> for Component<'a> {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        read_str_with_limit::<_, CHAT_LIMIT>(cursor)
            .and_then(|str| serde_json::from_str(str).map_err(|err| ProtocolError::Any(err.into())))
    }
}

delegate_size!(Identifier<'_> = &str);

impl<'a> ProtocolWritable for Identifier<'a> {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self.get_inner() {
            IdentifierInner::Full(full) => write_str_with_limit::<_, DEFAULT_LIMIT>(full, writer),
            IdentifierInner::Partial(key, value) => match key.len() + value.len() <= DEFAULT_LIMIT - 1 {
                true => {
                    VarInt::write_variant(&(key.len() as i32 + value.len() as i32 + 1), writer)?;
                    writer.write_bytes(key.as_bytes());
                    writer.write_byte(b':');
                    writer.write_bytes(value.as_bytes());
                    Ok(())
                }
                false => Err(too_long_string()),
            }
        }
    }
}

impl<'a> ProtocolReadable<'a> for Identifier<'a> {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        Identifier::new_full(Cow::Borrowed(read_str_with_limit::<_, DEFAULT_LIMIT>(cursor)?))
            .ok_or_else(|| ProtocolError::Any(anyhow::Error::msg("Bad identifier")))
    }
}

delegate_size!(Angle = u8);

impl ProtocolVariantWritable<f32> for Angle {
    fn write_variant<W: ProtocolWriter>(object: &f32, writer: &mut W) -> anyhow::Result<()> {
        ((*object * 256.0 / std::f32::consts::PI) as u8).write(writer)
    }
}

impl<'a> ProtocolVariantReadable<'a, f32> for Angle {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<f32> {
        Ok((u8::read(cursor)? as f32) * std::f32::consts::PI / 256.0)
    }
}

fixed_range_size!(Nbt = (1, u32::MAX));

#[cfg(feature = "fastnbt")]
mod fastnbt_impls {
    use super::*;

    impl<T: Serialize> ProtocolVariantWritable<T> for Nbt {
        fn write_variant<W: ProtocolWriter>(object: &T, writer: &mut W) -> anyhow::Result<()> {
            Ok(writer.write_vec_bytes(fastnbt::to_bytes(object)?))
        }
    }

    impl<'a, T: Deserialize<'a>> ProtocolVariantReadable<'a, T> for Nbt {
        fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<T> {
            fastnbt::from_reader(ReadableProtocolCursor::new(cursor))
                .map_err(|err| ProtocolError::Any(err.into()))
        }
    }
}

pub(crate) mod nbt {
    use super::*;

    pub struct ProtocolSkipCursor<'a, C: ProtocolCursor<'a>> {
        pub cursor: C,
        pub length: usize,
        _marker: PhantomData<&'a ()>,
    }

    impl<'a, C: ProtocolCursor<'a>> ProtocolSkipCursor<'a, C> {
        pub fn new(cursor: C) -> Self {
            Self {
                cursor,
                length: 0,
                _marker: PhantomData,
            }
        }

        fn skip(&mut self, length: usize) -> ProtocolResult<()> {
            self.cursor.take_bytes(length).map(|_| ())
        }
    }

    impl<'a, C: ProtocolCursor<'a>> ProtocolCursor<'a> for ProtocolSkipCursor<'a, C> {
        fn take_byte(&mut self) -> ProtocolResult<u8> {
            self.length += 1;
            self.cursor.take_byte()
        }

        fn take_bytes(&mut self, length: usize) -> ProtocolResult<&'a [u8]> {
            self.length += length;
            self.cursor.take_bytes(length)
        }

        fn remaining_bytes(&self) -> usize {
            self.cursor.remaining_bytes()
        }

        fn take_cursor(&self) -> Self {
            ProtocolSkipCursor {
                cursor: self.cursor.take_cursor(),
                length: self.length,
                _marker: PhantomData,
            }
        }

        fn has_bytes(&self, length: usize) -> bool {
            self.cursor.has_bytes(length)
        }
    }

    pub fn skip_string<'a, C: ProtocolCursor<'a>>(cursor: &mut ProtocolSkipCursor<'a, C>) -> ProtocolResult<()> {
        let length = u16::read(cursor)?;
        cursor.skip(length as usize)
    }

    pub fn skip_compound<'a, C: ProtocolCursor<'a>>(cursor: &mut ProtocolSkipCursor<'a, C>) -> ProtocolResult<()> {
        let tag = u8::read(cursor)?;
        if tag != 10 {
            return Err(ProtocolError::Any(anyhow::Error::msg("Nbt does not start with compound")));
        }
        skip_string(cursor)?;
        skip_entered_compound(cursor)
    }

    pub fn skip_entered_compound<'a, C: ProtocolCursor<'a>>(cursor: &mut ProtocolSkipCursor<'a, C>) -> ProtocolResult<()> {
        loop {
            let tag = u8::read(cursor)?;
            if tag == 0 { break; }
            skip_string(cursor)?;
            skip_tag(cursor, tag, 1)?;
        }
        Ok(())
    }

    pub fn skip_tag<'a, C: ProtocolCursor<'a>>(cursor: &mut ProtocolSkipCursor<'a, C>, tag: u8, times: usize) -> ProtocolResult<()> {
        match tag {
            0 => Ok(()),
            1 => cursor.skip(1 * times),
            2 => cursor.skip(2 * times),
            3 => cursor.skip(4 * times),
            4 => cursor.skip(8 * times),
            5 => cursor.skip(4 * times),
            6 => cursor.skip(8 * times),
            7 => {
                for _ in 0..times {
                    let length = i32::read(cursor)?;
                    cursor.skip(length as usize)?
                }
                Ok(())
            }
            8 => {
                for _ in 0..times {
                    skip_string(cursor)?
                }
                Ok(())
            }
            9 => {
                for _ in 0..times {
                    let tag = u8::read(cursor)?;
                    let times = i32::read(cursor)?;
                    skip_tag(cursor, tag, times as usize)?
                }
                Ok(())
            }
            10 => {
                for _ in 0..times {
                    skip_compound(cursor)?
                }
                Ok(())
            }
            11 => {
                for _ in 0..times {
                    let length = i32::read(cursor)?;
                    cursor.skip(length as usize * 4)?;
                }
                Ok(())
            }
            12 => {
                for _ in 0..times {
                    let length = i32::read(cursor)?;
                    cursor.skip(length as usize * 8)?;
                }
                Ok(())
            }
            _ => Err(ProtocolError::Any(anyhow::Error::msg("Bad nbt tag"))),
        }
    }
}

impl ProtocolSize for NbtBytes { const SIZE: Range<u32> = Nbt::SIZE; }

impl ProtocolVariantWritable<[u8]> for NbtBytes {
    fn write_variant<W: ProtocolWriter>(object: &[u8], writer: &mut W) -> anyhow::Result<()> {
        Ok(writer.write_bytes(object))
    }
}

impl<'a> ProtocolVariantWritable<&'a [u8]> for NbtBytes {
    fn write_variant<W: ProtocolWriter>(object: &&'a [u8], writer: &mut W) -> anyhow::Result<()> {
        Self::write_variant(*object, writer)
    }
}

impl ProtocolVariantWritable<Vec<u8>> for NbtBytes {
    fn write_variant<W: ProtocolWriter>(object: &Vec<u8>, writer: &mut W) -> anyhow::Result<()> {
        Self::write_variant(object.as_slice(), writer)
    }
}

impl<'a> ProtocolVariantWritable<Cow<'a, [u8]>> for NbtBytes {
    fn write_variant<W: ProtocolWriter>(object: &Cow<'a, [u8]>, writer: &mut W) -> anyhow::Result<()> {
        match object {
            Cow::Owned(owned) => Self::write_variant(owned, writer),
            Cow::Borrowed(borrowed) => Self::write_variant(borrowed, writer),
        }
    }
}

impl<'a> ProtocolVariantReadable<'a, &'a [u8]> for NbtBytes {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<&'a [u8]> {
        let mut skip_cursor = nbt::ProtocolSkipCursor::new(cursor.take_cursor());
        nbt::skip_compound(&mut skip_cursor)?;
        cursor.take_bytes(skip_cursor.length)
    }
}

impl<'a> ProtocolVariantReadable<'a, Vec<u8>> for NbtBytes {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Vec<u8>> {
        Self::read_variant(cursor).map(|slice: &'a [u8]| slice.to_owned())
    }
}

impl<'a> ProtocolVariantReadable<'a, Cow<'a, [u8]>> for NbtBytes {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Cow<'a, [u8]>> {
        Self::read_variant(cursor).map(|slice| Cow::Borrowed(slice))
    }
}

delegate_size!(BlockPosition = u64);

#[cfg(feature = "euclid")]
mod euclid_impls {
    use super::*;

    impl<T: ProtocolSize, U> ProtocolSize for Vector3D<T, U> {
        const SIZE: Range<u32> = (
            add_u32_without_overflow_array([T::SIZE.start; 3])..
                add_u32_without_overflow_array([T::SIZE.end; 3])
        );
    }

    impl<T: ProtocolSize, U> ProtocolSize for Vector2D<T, U> {
        const SIZE: Range<u32> = (
            add_u32_without_overflow_array([T::SIZE.start; 2])..
                add_u32_without_overflow_array([T::SIZE.end; 2])
        );
    }

    impl<T: ProtocolWritable, U> ProtocolWritable for Vector3D<T, U> {
        fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
            self.x.write(writer)?;
            self.y.write(writer)?;
            self.z.write(writer)
        }
    }

    impl<T: ProtocolWritable, U> ProtocolWritable for Vector2D<T, U> {
        fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
            self.x.write(writer)?;
            self.y.write(writer)
        }
    }

    impl<U> ProtocolVariantWritable<Vector3D<i32, U>> for BlockPosition {
        fn write_variant<W: ProtocolWriter>(object: &Vector3D<i32, U>, writer: &mut W) -> anyhow::Result<()> {
            (((object.x as i64 & 0x3FFFFFF) << 38) |
                ((object.z as i64 & 0x3FFFFFF) << 12) |
                (object.y as i64 & 0xFFF)
            ).write(writer)
        }
    }

    impl<'a, T: ProtocolReadable<'a>, U: 'a> ProtocolReadable<'a> for Vector3D<T, U> {
        fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
            Ok(Self {
                x: T::read(cursor)?,
                y: T::read(cursor)?,
                z: T::read(cursor)?,
                _unit: PhantomData,
            })
        }
    }

    impl<'a, T: ProtocolReadable<'a>, U: 'a> ProtocolReadable<'a> for Vector2D<T, U> {
        fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
            Ok(Self {
                x: T::read(cursor)?,
                y: T::read(cursor)?,
                _unit: PhantomData,
            })
        }
    }

    impl<'a, U: 'a> ProtocolVariantReadable<'a, Vector3D<i32, U>> for BlockPosition {
        fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Vector3D<i32, U>> {
            let value = u64::read(cursor)?;
            let mut x = (value >> 38) as i32;
            let mut y = (value & 0xFFF) as i32;
            let mut z = ((value >> 12) & 0x3FFFFFF) as i32;
            if x >= 0x2000000 {
                x -= 0x4000000
            }
            if y >= 0x800 {
                y -= 0x1000
            }
            if z >= 0x2000000 {
                z -= 0x4000000
            }
            Ok(Vector3D { x, y, z, _unit: PhantomData })
        }
    }
}

impl<T, const N: u8> ProtocolSize for FixedPointNumber<T, N>
    where T: ProtocolSize {
    const SIZE: Range<u32> = T::SIZE;
}

macro_rules! fixed_point_number_impl {
    ($($ty: ty$(,)*)*) => {
        $(
        impl<'a, const N: u8> ProtocolVariantReadable<'a, f32> for FixedPointNumber<$ty, N> {
            fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<f32> {
                Ok((<$ty>::read(cursor)? as f32) / (1 << N) as f32)
            }
        }

        impl<'a, const N: u8> ProtocolVariantReadable<'a, f64> for FixedPointNumber<$ty, N> {
            fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<f64> {
                Ok((<$ty>::read(cursor)? as f64) / (1 << N) as f64)
            }
        }

        impl<const N: u8> ProtocolVariantWritable<f32> for FixedPointNumber<$ty, N> {
            fn write_variant<W: ProtocolWriter>(object: &f32, writer: &mut W) -> anyhow::Result<()> {
                ((*object * (1 << N) as f32) as $ty).write(writer)
            }
        }

        impl<const N: u8> ProtocolVariantWritable<f64> for FixedPointNumber<$ty, N> {
            fn write_variant<W: ProtocolWriter>(object: &f64, writer: &mut W) -> anyhow::Result<()> {
                ((*object * (1 << N) as f64) as $ty).write(writer)
            }
        }
        )*
    }
}

fixed_point_number_impl!(i16, u16, i32, u32, i64, u64);

impl ProtocolCursorIteratorLimiter for ProtocolCursorIteratorCountLimiter {
    fn next(&mut self) -> bool {
        if self.count == 0 { return false; }
        self.count -= 1;
        true
    }
}

impl ProtocolCursorIteratorLimiter for ProtocolCursorIteratorNoLimiter {
    fn next(&mut self) -> bool {
        true
    }
}

impl<'a, 'b, C, L, V, VV> ProtocolCursorIterator<'a, 'b, C, L, V, VV> {
    pub fn new(cursor: &'a mut C, limiter: L) -> Self {
        Self {
            cursor,
            limiter,
            _marker: PhantomData,
        }
    }
}

impl<'a, 'b, C: ProtocolCursor<'b>, L: ProtocolCursorIteratorLimiter, V, VV: ProtocolVariantReadable<'b, V>> Iterator for ProtocolCursorIterator<'a, 'b, C, L, V, VV> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.limiter.next() { return None; }
        match VV::read_variant(self.cursor) {
            Ok(val) => Some(val),
            Err(ProtocolError::End) => None,
            Err(ProtocolError::Any(err)) => {
                // TODO think about error handling
                if cfg!(debug_assertions) {
                    panic!("{}", err);
                } else {
                    None
                }
            }
        }
    }
}

impl<T: ProtocolSize, const S: usize> ProtocolSize for ProtocolSizeOption<T, S> {
    const SIZE: Range<u32> = min_u32(T::SIZE.start, S as _)..max_u32(T::SIZE.end, S as _);
}

impl<'a, T: ProtocolReadable<'a>, const S: usize> ProtocolVariantReadable<'a, Option<T>> for ProtocolSizeOption<T, S> {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Option<T>> {
        match cursor.remaining_bytes() == S {
            true => Ok(None),
            false => T::read(cursor).map(|val| Some(val))
        }
    }
}

impl<T: ProtocolWritable, const S: usize> ProtocolVariantWritable<Option<T>> for ProtocolSizeOption<T, S> {
    fn write_variant<W: ProtocolWriter>(object: &Option<T>, writer: &mut W) -> anyhow::Result<()> {
        match object {
            Some(val) => val.write(writer),
            None => Ok(writer.write_fixed_bytes([0; S]))
        }
    }
}

impl<T: ProtocolSize, const LENGTH: usize> ProtocolSize for ConstLengthArray<T, LENGTH> {
    const SIZE: Range<u32> = add_u32_without_overflow_array([T::SIZE.start; LENGTH])..add_u32_without_overflow_array([T::SIZE.end; LENGTH]);
}

impl<T: ProtocolWritable, const LENGTH: usize> ProtocolVariantWritable<[T; LENGTH]> for ConstLengthArray<T, LENGTH> {
    fn write_variant<W: ProtocolWriter>(object: &[T; LENGTH], writer: &mut W) -> anyhow::Result<()> {
        for obj in object {
            obj.write(writer)?
        }
        Ok(())
    }
}

impl<'a, T: ProtocolReadable<'a> + Clone, const LENGTH: usize> ProtocolVariantReadable<'a, [T; LENGTH]> for ConstLengthArray<T, LENGTH> {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<[T; LENGTH]> {
        let mut result: [T; LENGTH] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut current = result.as_mut_slice();
        for _ in 0..LENGTH {
            current[0] = T::read(cursor)?;
            current = &mut current[1..];
        }
        Ok(result)
    }
}

impl<T: ProtocolSize, const LENGTH: usize> ProtocolSize for ConstLengthRawArray<T, LENGTH> {
    const SIZE: Range<u32> = add_u32_without_overflow_array([T::SIZE.start; LENGTH])..add_u32_without_overflow_array([T::SIZE.end; LENGTH]);
}

impl<'a, T: ProtocolWritable + ProtocolRaw, const LENGTH: usize> ProtocolVariantWritable<&'a [T; LENGTH]> for ConstLengthRawArray<T, LENGTH> {
    fn write_variant<W: ProtocolWriter>(object: &&'a [T; LENGTH], writer: &mut W) -> anyhow::Result<()> {
        Ok(writer.write_bytes(t_array_into_byte_array(*object)))
    }
}

impl<T, const LENGTH: usize> ConstLengthRawArray<T, LENGTH> {
    const BYTES_LENGTH: usize = LENGTH * size_of::<T>();
}

impl<'a, T: ProtocolReadable<'a> + ProtocolRaw, const LENGTH: usize> ProtocolVariantReadable<'a, &'a [T; LENGTH]> for ConstLengthRawArray<T, LENGTH>
    where [(); Self::BYTES_LENGTH ]: {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<&'a [T; LENGTH]> {
        let bytes = cursor.take_fixed_bytes::<{ Self::BYTES_LENGTH }>()?;
        Ok(unsafe { &*(bytes as *const u8 as *const () as *const [T; LENGTH]) })
    }
}

impl<V, VV: ProtocolSize> ProtocolSize for ProtocolVariantOption<V, VV> {
    const SIZE: Range<u32> = 1..add_u32_without_overflow(VV::SIZE.end, 1);
}

impl<V, VV: ProtocolVariantWritable<V>> ProtocolVariantWritable<Option<V>> for ProtocolVariantOption<V, VV> {
    fn write_variant<W: ProtocolWriter>(object: &Option<V>, writer: &mut W) -> anyhow::Result<()> {
        match object {
            Some(val) => {
                true.write(writer)?;
                VV::write_variant(val, writer)
            },
            None => false.write(writer),
        }
    }
}

impl<'a, V, VV: ProtocolVariantReadable<'a, V>> ProtocolVariantReadable<'a, Option<V>> for ProtocolVariantOption<V, VV> {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Option<V>> {
        Ok(match bool::read(cursor)? {
            true => Some(VV::read_variant(cursor)?),
            false => None,
        })
    }
}
