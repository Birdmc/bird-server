use proc_macro2::{Span, TokenStream, Ident};
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};
use crate::shared::obligate_lifetime;

pub fn impl_derive(item: proc_macro::TokenStream) -> syn::Result<TokenStream> {
    let item: DeriveInput = syn::parse(item)?;
    let DeriveInput {
        ident,
        mut generics,
        data,
        attrs,
        ..
    } = item;
    let (lifetime, spec_impl_generics) = obligate_lifetime(&mut generics)?;
    let (write_prepare, read_create, fields) = if let Data::Struct(data_struct) = data {
        match data_struct.fields {
            Fields::Unit => (quote! {}, quote! { Self }, Vec::new()),
            Fields::Named(named) => {
                let idents: Vec<_> = named.named.iter().map(|field| field.ident.clone().unwrap()).collect();
                (
                    quote! { let Self { #(#idents,)* } = self },
                    quote! { Self { #(#idents,)* } },
                    named.named.into_iter().map(|field| (field.ident.unwrap(), field.ty)).collect()
                )
            },
            Fields::Unnamed(_) => Err(syn::Error::new(Span::call_site(), "Unnamed not supported"))?
        }
    } else { Err(syn::Error::new(Span::call_site(), "Only struct supported"))? };
    /*
    pub trait ProtocolNbtTag<'a>: 'a {
        const TAG: i8;
        const SIZE: Range<u32>;

        fn default_nbt() -> Option<Self> {
            None
        }

        fn skip_nbt<C: ProtocolCursor<'a>>(cursor: &mut C, amount: usize) -> ProtocolResult<usize>;

        fn write_nbt<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()>;

        fn read_nbt<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self>;
    }
    */
    let (_impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let (impl_generics, ..) = spec_impl_generics.split_for_impl();
    let writes = fields.iter().map(|(ident, ty)| {
        let ident_str = ident.to_string();
        quote! {
            <_ as bird_protocol::nbt::ProtocolNbtTag<#lifetime>>::write_nbt(&<#ty as bird_protocol::nbt::ProtocolNbtTag<#lifetime>>::NBT_TAG, __writer)?;
            <_ as bird_protocol::nbt::ProtocolNbtTag<#lifetime>>::write_nbt(&std::borrow::Cow::Borrowed(#ident_str), __writer)?;
            <_ as bird_protocol::nbt::ProtocolNbtTag<#lifetime>>::write_nbt(#ident, __writer)?;
        }
    });
    let read_fields = fields.iter().map(|(ident, ty)|
        quote! { let mut #ident: std::option::Option<#ty> = std::option::Option::None; }
    );
    let read_matches = fields.iter().map(|(ident, ty)| {
        let ident_str = ident.to_string();
        quote! {
            #ident_str => {
                if __tag != <#ty as bird_protocol::nbt::ProtocolNbtTag<#lifetime>>::NBT_TAG {
                    bird_protocol::ProtocolResult::Err(bird_protocol::ProtocolError::Any(bird_protocol::anyhow::Error::msg("Not right tag")))?;
                }
                #ident.replace(<#ty as bird_protocol::nbt::ProtocolNbtTag<#lifetime>>::read_nbt(__cursor)?);
            },
        }
    });
    let read_post_fields = fields.iter().map(|(ident, ty)| {
        quote! { let #ident = #ident
            .or_else(|| <#ty as bird_protocol::nbt::ProtocolNbtTag<#lifetime>>::default_nbt())
            .ok_or_else(|| bird_protocol::ProtocolError::Any(bird_protocol::anyhow::Error::msg("One variable is not presented")))?;
        }
    });
    Ok(quote! {
        impl #impl_generics bird_protocol::nbt::ProtocolNbtTag<#lifetime> for #ident #ty_generics #where_clause {
            const NBT_TAG: u8 = bird_protocol::nbt::COMPOUND_TAG;
            const NBT_SIZE: std::ops::Range<u32> = (0..0);

            fn skip_nbt<C: bird_protocol::ProtocolCursor<#lifetime>>(__cursor: &mut C, __amount: usize) -> bird_protocol::ProtocolResult<usize> {
                let mut result = 0usize;
                for _ in 0..__amount { result += bird_protocol::nbt::skip_compound(__cursor)? }
                Ok(result)
            }

            fn write_nbt<W: bird_protocol::ProtocolWriter>(&self, __writer: &mut W) -> bird_protocol::anyhow::Result<()> {
                #write_prepare;
                #(#writes)*
                <u8 as bird_protocol::nbt::ProtocolNbtTag<#lifetime>>::write_nbt(&0, __writer)?;
                Ok(())
            }

            fn read_nbt<C: bird_protocol::ProtocolCursor<#lifetime>>(__cursor: &mut C) -> bird_protocol::ProtocolResult<Self> {
                #(#read_fields)*
                bird_protocol::nbt::read_compound(__cursor, |__tag, __name, __cursor| {
                    match __name {
                        #(#read_matches)*
                        _ => bird_protocol::ProtocolResult::Err(bird_protocol::ProtocolError::Any(bird_protocol::anyhow::Error::msg("Bad compound variable")))?,
                    };
                    bird_protocol::ProtocolResult::Ok(())
                })?;
                #(#read_post_fields)*
                bird_protocol::ProtocolResult::Ok(#read_create)
            }
        }
    })
}