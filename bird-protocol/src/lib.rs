use std::{ops::Range, marker::PhantomData};

mod impls;
mod stdimpls;

pub use crate::stdimpls::StdIOReadProtocolCursor as ReadableProtocolCursor;

pub mod __private {
    pub use crate::impls::*;
}

#[cfg(feature = "derive")]
pub mod derive {
    pub use bird_protocol_macro::*;
}

pub struct VarInt;

pub struct VarLong;

pub struct RemainingBytesArray;

pub struct RemainingArray<V, VV>(PhantomData<(V, VV)>);

pub struct LengthProvidedBytesArray<L, LV>(PhantomData<(L, LV)>);

pub struct LengthProvidedArray<L, LV, V, VV>(PhantomData<(L, LV, V, VV)>);

pub struct Json;

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

pub trait ProtocolSize {
    const SIZE: Range<u32>;
}

pub trait ProtocolCursor<'a> {
    fn take_byte(&mut self) -> ProtocolResult<u8>;

    fn take_bytes(&mut self, length: usize) -> ProtocolResult<&'a [u8]>;

    fn remaining_bytes(&self) -> usize;

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
