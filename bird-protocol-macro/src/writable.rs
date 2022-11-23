use proc_macro2::TokenStream;
use quote::quote;

pub fn impl_derive(item: proc_macro::TokenStream) -> syn::Result<TokenStream> {
    Ok(quote! {})
}