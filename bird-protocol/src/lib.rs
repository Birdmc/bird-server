#![feature(generic_const_exprs)]

use std::{ops::Range, marker::PhantomData};

mod impls;
mod std_impls;
mod pub_impls;
#[cfg(feature = "birdnbt")]
pub mod nbt;

pub use pub_impls::*;

pub use crate::std_impls::StdIOReadProtocolCursor as ReadableProtocolCursor;

pub use anyhow;

#[doc(hidden)]
pub mod __private {
    pub use crate::impls::*;
}

pub use crate::impls::add_protocol_sizes;

#[cfg(feature = "derive")]
pub mod derive {
    pub use bird_protocol_macro::*;
}

pub struct VarInt;

pub struct VarLong;

pub type LengthFunctionBytesArray<T> = LengthFunctionRawArray<u8, u8, T>;

pub struct LengthFunctionRawArray<V, VV, T>(PhantomData<(V, VV, T)>);

pub struct LengthFunctionArray<V, VV, T>(PhantomData<(V, VV, T)>);

pub type RemainingBytesArray = RemainingRawArray<u8, u8>;

pub type RemainingRawArray<V, VV> = LengthFunctionRawArray<V, VV, ProtocolLengthRemainingDeterminer>;

pub type RemainingArray<V, VV> = LengthFunctionArray<V, VV, ProtocolLengthRemainingDeterminer>;

pub type LengthProvidedBytesArray<L, LV> = LengthProvidedRawArray<L, LV, u8, u8>;

pub type LengthProvidedRawArray<L, LV, V, VV> = LengthFunctionRawArray<V, VV, ProtocolLengthProvidedDeterminer<L, LV>>;

pub type LengthProvidedArray<L, LV, V, VV> = LengthFunctionArray<V, VV, ProtocolLengthProvidedDeterminer<L, LV>>;

pub type LengthConstBytesArray<const SIZE: usize> = LengthConstRawArray<u8, u8, SIZE>;

pub type LengthConstRawArray<V, VV, const SIZE: usize> = LengthFunctionRawArray<V, VV, ProtocolLengthConstDeterminer<SIZE>>;

pub type LengthConstArray<V, VV, const SIZE: usize> = LengthFunctionArray<V, VV, ProtocolLengthConstDeterminer<SIZE>>;

pub struct ConstLengthArray<T, const LENGTH: usize>(PhantomData<T>);

pub struct ConstLengthRawArray<T, const LENGTH: usize>(PhantomData<T>);

pub struct ProtocolVariantOption<V, VV>(PhantomData<(V, VV)>);

pub struct Json;

pub struct Nbt;

pub struct NbtBytes;

pub struct Angle;

pub struct BlockPosition;

pub struct FixedPointNumber<T, const N: u8>(PhantomData<T>,);

pub trait ProtocolLengthDeterminer<'a>: ProtocolVariantReadable<'a, usize> + ProtocolVariantWritable<usize> + ProtocolSize {
    const ELEMENT_COUNT: bool;
}

pub struct ProtocolLengthProvidedDeterminer<L, LV>(PhantomData<(L, LV)>);

pub struct ProtocolLengthRemainingDeterminer;

pub struct ProtocolLengthConstDeterminer<const N: usize>;

pub trait ProtocolCursorIteratorLimiter {
    fn next(&mut self) -> bool;
}

pub struct ProtocolCursorIteratorCountLimiter {
    pub count: usize,
}

pub struct ProtocolCursorIteratorNoLimiter;

pub struct ProtocolCursorIterator<'a, 'b, C, L, V, VV> {
    cursor: &'a mut C,
    limiter: L,
    _marker: PhantomData<&'b (V, VV)>,
}

pub struct ProtocolSizeOption<T, const SIZE: usize>(PhantomData<T>);

pub trait ProtocolLength {
    fn into_usize(self) -> usize;

    fn from_usize(size: usize) -> Self;
}

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("Tried to take too many bytes")]
    End,
    #[error("Any: {0:?}")]
    Any(#[from] anyhow::Error),
}

pub type ProtocolResult<T> = Result<T, ProtocolError>;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ProtocolPacketBound {
    Client,
    Server,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ProtocolPacketState {
    Handshake,
    Status,
    Login,
    Play,
}

pub trait ProtocolPacket {
    const ID: i32;
    const BOUND: ProtocolPacketBound;
    const STATE: ProtocolPacketState;
}

pub unsafe trait ProtocolRaw {}

pub trait ProtocolSize {
    const SIZE: Range<u32>;
}

pub trait ProtocolCursor<'a> {
    fn take_byte(&mut self) -> ProtocolResult<u8>;

    /// # Features
    /// Returned slice must be with length of the given length
    fn take_bytes(&mut self, length: usize) -> ProtocolResult<&'a [u8]>;

    fn take_fixed_bytes<const LENGTH: usize>(&mut self) -> ProtocolResult<&'a [u8; LENGTH]> {
        self.take_bytes(LENGTH)?
            .try_into()
            .map_err(|err| ProtocolError::Any(anyhow::Error::msg("Something bad happened")))
    }

    fn remaining_bytes(&self) -> usize;

    /// # Features
    /// Took cursor should be the same as `self`
    fn take_cursor(&self) -> Self;

    fn has_bytes(&self, length: usize) -> bool {
        length <= self.remaining_bytes()
    }
}

pub trait ProtocolWriter {
    fn write_bytes(&mut self, bytes: &[u8]);

    fn write_byte(&mut self, byte: u8) {
        self.write_fixed_bytes([byte])
    }

    fn write_fixed_bytes<const SIZE: usize>(&mut self, bytes: [u8; SIZE]) {
        self.write_bytes(bytes.as_slice())
    }

    fn write_vec_bytes(&mut self, bytes: Vec<u8>) {
        self.write_bytes(bytes.as_slice())
    }
}

pub trait ProtocolWritable: ProtocolSize {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()>;
}

pub trait ProtocolVariantWritable<V: ?Sized>: ProtocolSize {
    fn write_variant<W: ProtocolWriter>(
        object: &V,
        writer: &mut W,
    ) -> anyhow::Result<()>;
}

pub trait ProtocolReadable<'a>: ProtocolSize + Sized + 'a {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self>;
}

pub trait ProtocolVariantReadable<'a, V>: ProtocolSize {
    fn read_variant<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<V>;
}
