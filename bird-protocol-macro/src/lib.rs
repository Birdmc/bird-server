mod readable;
mod shared;
mod writable;
mod size;

use writable::impl_derive as protocol_writable_derive_impl;
use readable::impl_derive as protocol_readable_derive_impl;
use size::impl_derive as protocol_size_derive_impl;

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
    derive_impl!(protocol_writable_derive_impl(item))
}

#[proc_macro_derive(ProtocolReadable, attributes(bp))]
pub fn protocol_readable_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_impl!(protocol_readable_derive_impl(item))
}

#[proc_macro_derive(ProtocolSize, attributes(bp))]
pub fn protocol_size_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_impl!(protocol_size_derive_impl(item))
}