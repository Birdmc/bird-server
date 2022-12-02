use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::DeriveInput;
use crate::shared::{ObjectAttributes, parse_attributes};

pub fn impl_derive(item: proc_macro::TokenStream) -> syn::Result<TokenStream> {
    let item: DeriveInput = syn::parse(item)?;
    let DeriveInput {
        ident,
        generics,
        attrs,
        ..
    } = item;
    let object_attributes: ObjectAttributes = parse_attributes(&attrs, "bp")?;
    let id = object_attributes.packet_id.ok_or_else(|| syn::Error::new(Span::call_site(), "packet id should be provided"))?;
    let state = object_attributes.packet_state.ok_or_else(|| syn::Error::new(Span::call_site(), "packet state should be provided"))?;
    let bound = object_attributes.packet_bound.ok_or_else(|| syn::Error::new(Span::call_site(), "packet bound should be provided"))?;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics bird_protocol::ProtocolPacket for #ident #type_generics #where_clause {
            const ID: i32 = #id;
            const BOUND: bird_protocol::ProtocolPacketBound = #bound;
            const STATE: bird_protocol::ProtocolPacketState = #state;
        }
    })
}