use byteorder::{WriteBytesExt, ReadBytesExt, BigEndian};
use std::io::{Write, Result, Read};

pub type MinecraftEndian = BigEndian;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProtocolSizeRange {
    min: u32,
    max: u32,
}

pub trait ProtocolSize {
    const SIZE: ProtocolSizeRange;
}

pub trait ProtocolWritable: ProtocolSize {
    fn write<W: Write + WriteBytesExt>(&self, write: &mut W) -> Result<()>; 
}

pub trait ProtocolReadable: ProtocolSize + Sized {
    fn read<R: Read + ReadBytesExt>(read: &mut R) -> Result<Self>;
}

pub const fn add_size_range(first: ProtocolSizeRange, second: ProtocolSizeRange) -> ProtocolSizeRange {
    ProtocolSizeRange { 
        min: first.min + second.min, 
        max: match u32::MAX - first.max < second.max { 
            true => u32::MAX, 
            false => first.max + second.max 
        }
    }
}

macro_rules! size_of_native_impl {
    ($ty: ty) => {
        impl $crate::ProtocolSize for $ty {
            const SIZE: ProtocolSizeRange = ProtocolSizeRange { min: std::mem::size_of::<Self>() as u32, max: std::mem::size_of::<Self>() as u32};
        }
    }
}

macro_rules! number_impl {
    ($ty: ty, $write_func: ident, $read_func: ident$(,)* $($generics: ident $(,)*)*) => {
        size_of_native_impl!($ty);

        impl $crate::ProtocolWritable for $ty {
            fn write<W: std::io::Write + byteorder::WriteBytesExt>(&self, write: &mut W) -> std::io::Result<()> {
                write. $write_func ::<$($generics,)*>(*self)
            }
        }

        impl $crate::ProtocolReadable for $ty {
            fn read<R: std::io::Read + byteorder::ReadBytesExt>(read: &mut R) -> std::io::Result<Self> {
                read. $read_func ::<$($generics,)*>()
            }
        }
    }
}

number_impl!(u8, write_u8, read_u8);
number_impl!(i8, write_i8, read_i8);
number_impl!(u16, write_u16, read_u16, MinecraftEndian);
number_impl!(i16, write_i16, read_i16, MinecraftEndian);
number_impl!(i32, write_i32, read_i32, MinecraftEndian);
number_impl!(u32, write_u32, read_u32, MinecraftEndian);
number_impl!(i64, write_i64, read_i64, MinecraftEndian);
number_impl!(u64, write_u64, read_u64, MinecraftEndian);
number_impl!(f32, write_f32, read_f32, MinecraftEndian);
number_impl!(f64, write_f64, read_f64, MinecraftEndian);
size_of_native_impl!(bool);

impl ProtocolWritable for bool {
    fn write<W: Write + WriteBytesExt>(&self, write: &mut W) -> Result<()> {
        write.write_u8(*self as u8)
    }
}

impl ProtocolReadable for bool {
    fn read<R: Read + ReadBytesExt>(read: &mut R) -> Result<Self> {
        read.read_u8().map(|num| num != 0)
    }
}

impl<T: ProtocolSize> ProtocolSize for Option<T> {
    const SIZE: ProtocolSizeRange = ProtocolSizeRange { min: 1, max: 1 + T::SIZE.max };
}

impl<T: ProtocolReadable> ProtocolReadable for Option<T> {
    fn read<R: Read + ReadBytesExt>(read: &mut R) -> Result<Self> {
        Ok(match bool::read(read)? {
            true => Some(T::read(read)?),
            false => None
        })
    }
}

impl<T: ProtocolWritable> ProtocolWritable for Option<T> {
    fn write<W: Write + WriteBytesExt>(&self, write: &mut W) -> Result<()> {
        match self {
            Some(obj) => {
                true.write(write)?;
                obj.write(write)
            },
            None => false.write(write) 
        }
    }
}