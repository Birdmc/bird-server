use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Field, Fields, parse_macro_input};
use crate::shared::{create_prepared_fields, obligate_lifetime};

pub fn impl_derive(item: proc_macro::TokenStream) -> syn::Result<TokenStream> {
    let item: DeriveInput = syn::parse(item)?;
    let DeriveInput {
        data,
        ident,
        mut generics,
        ..
    } = item;
    let (lifetime, spec_impl_generics) = obligate_lifetime(&mut generics)?;
    let function_body = match data {
        Data::Struct(data_struct) => {
            let create_struct_ts = match data_struct.fields {
                Fields::Unit => quote! { Ok(Self) },
                Fields::Unnamed(ref unnamed) => {
                    let mut idents = Vec::new();
                    for i in 0..unnamed.unnamed.len() {
                        idents.push(Ident::new(format!("__{}", i).as_str(), Span::call_site()));
                    }
                    quote! { Ok(Self(#(#idents,)*)) }
                }
                Fields::Named(ref named) => {
                    let mut idents = Vec::new();
                    for field in &named.named {
                        idents.push(field.ident.as_ref().unwrap().to_token_stream())
                    }
                    quote! { Ok(Self{#(#idents,)*}) }
                }
            };
            let fields = create_prepared_fields(data_struct.fields)?;
            let mut variables_ts = Vec::new();
            for (field, field_attribute) in fields {
                let Field {
                    ident,
                    ty,
                    ..
                } = field;
                variables_ts.push(match field_attribute.variant {
                    Some(variant) => quote! { let #ident = <#variant as bird_protocol::ProtocolVariantReadable<#lifetime, #ty>>::read_variant(cursor)? },
                    None => quote! { let #ident = <#ty as bird_protocol::ProtocolReadable<#lifetime>>::read(cursor)? },
                })
            }
            quote! {
                #(#variables_ts;)*
                #create_struct_ts
            }
        }
        Data::Enum(data_enum) => unimplemented!(),
        Data::Union(_) => return Err(syn::Error::new(Span::mixed_site(), "Union is not supported")),
    };
    let (_, type_generics, where_clause) = generics.split_for_impl();
    let (impl_generics, ..) = spec_impl_generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics bird_protocol::ProtocolReadable<#lifetime> for #ident #type_generics #where_clause {
            fn read<C: bird_protocol::ProtocolCursor<#lifetime>>(cursor: &mut C) -> bird_protocol::ProtocolResult<Self> {
                #function_body
            }
        }
    })
}