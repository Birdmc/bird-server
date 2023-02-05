mod readable;
mod shared;
mod writable;
mod size;
mod packet;
mod nbt;

macro_rules! derive_impl {
    ($func: expr) => {
        match $func {
            Ok(res) => res,
            Err(err) => err.into_compile_error(),
        }.into()
    }
}

#[proc_macro_derive(ProtocolWritable, attributes(bp))]
pub fn protocol_writable_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_impl!(writable::impl_derive(item))
}

#[proc_macro_derive(ProtocolReadable, attributes(bp))]
pub fn protocol_readable_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_impl!(readable::impl_derive(item))
}

#[proc_macro_derive(ProtocolSize, attributes(bp))]
pub fn protocol_size_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_impl!(size::impl_derive(item))
}

#[proc_macro_derive(ProtocolPacket, attributes(bp))]
pub fn protocol_packet_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_impl!(packet::impl_derive(item))
}

#[proc_macro_derive(ProtocolAll, attributes(bp))]
pub fn protocol_all_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut writable: proc_macro::TokenStream = derive_impl!(writable::impl_derive(item.clone()));
    let readable: proc_macro::TokenStream = derive_impl!(readable::impl_derive(item.clone()));
    let size: proc_macro::TokenStream = derive_impl!(size::impl_derive(item));
    writable.extend(readable.into_iter());
    writable.extend(size.into_iter());
    writable
}

#[proc_macro_derive(BirdNbt, attributes(bnbt))]
pub fn bird_nbt_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // println!("{}", nbt::impl_derive(item).unwrap());
    // proc_macro::TokenStream::new()
    derive_impl!(nbt::impl_derive(item))
}