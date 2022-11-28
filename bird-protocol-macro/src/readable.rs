use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Field, Fields, parse_macro_input, Variant};
use crate::shared::{create_prepared_fields, create_prepared_variants, ObjectAttributes, obligate_lifetime, parse_attributes};

pub fn impl_derive(item: proc_macro::TokenStream) -> syn::Result<TokenStream> {
    let item: DeriveInput = syn::parse(item)?;
    let DeriveInput {
        attrs,
        data,
        ident,
        mut generics,
        ..
    } = item;
    let object_attributes: ObjectAttributes = parse_attributes(&attrs, "bp")?;
    let (lifetime, spec_impl_generics) = obligate_lifetime(&mut generics)?;
    let function_body = match data {
        Data::Struct(data_struct) => read_fields(data_struct.fields, quote! { Self }, &lifetime)?,
        Data::Enum(data_enum) => {
            let key_ty = object_attributes.key_ty.as_ref().ok_or_else(|| syn::Error::new(Span::call_site(), "You should provide key_ty for enum object"))?;
            let variants = create_prepared_variants(data_enum.variants.into_iter(), &object_attributes)?;
            let mut const_variant_values = Vec::new();
            let mut variant_matches = Vec::new();
            let mut const_match_value_counter = 0;
            for (variant, variant_value, _variant_attributes) in variants {
                let Variant {
                    fields,
                    ident,
                    ..
                } = variant;
                let variant_fields = read_fields(fields, quote! { Self:: #ident }, &lifetime)?;
                let const_match_value = Ident::new(format!("__{}", const_match_value_counter).as_str(), Span::call_site());
                const_match_value_counter += 1;
                const_variant_values.push(quote! { const #const_match_value: #key_ty = #variant_value });
                variant_matches.push(quote! {
                    #const_match_value => { #variant_fields }
                })
            }
            let key_read_ts = read_ts(&key_ty, &lifetime, object_attributes.key_variant.as_ref());
            quote! {
                #(#const_variant_values;)*
                match #key_read_ts {
                    #(#variant_matches,)*
                    _ => bird_protocol::ProtocolResult::Err(bird_protocol::ProtocolError::Any(bird_protocol::anyhow::Error::msg("Bad value of key"))),
                }
            }
        },
        Data::Union(_) => return Err(syn::Error::new(Span::mixed_site(), "Union is not supported")),
    };
    let (_, type_generics, where_clause) = generics.split_for_impl();
    let (impl_generics, ..) = spec_impl_generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics bird_protocol::ProtocolReadable<#lifetime> for #ident #type_generics #where_clause {
            fn read<C: bird_protocol::ProtocolCursor<#lifetime>>(__cursor: &mut C) -> bird_protocol::ProtocolResult<Self> {
                #function_body
            }
        }
    })
}

fn read_fields(fields: Fields, key: TokenStream, lifetime: &impl ToTokens) -> syn::Result<TokenStream> {
    let create_struct_ts = match fields {
        Fields::Unit => quote! { Ok(#key) },
        Fields::Unnamed(ref unnamed) => {
            let mut idents = Vec::new();
            for i in 0..unnamed.unnamed.len() {
                idents.push(Ident::new(format!("__{}", i).as_str(), Span::call_site()));
            }
            quote! { Ok(#key(#(#idents,)*)) }
        }
        Fields::Named(ref named) => {
            let mut idents = Vec::new();
            for field in &named.named {
                idents.push(field.ident.as_ref().unwrap())
            }
            quote! { Ok(#key{#(#idents,)*}) }
        }
    };
    let fields = create_prepared_fields(fields)?;
    let mut variables_ts = Vec::new();
    for (field, field_attribute) in fields {
        let Field {
            ident,
            ty,
            ..
        } = field;
        let read_ts = read_ts(&ty, lifetime, field_attribute.variant.as_ref());
        variables_ts.push(quote! { let #ident = #read_ts; });
    }
    Ok(quote! {
        #(#variables_ts;)*
        #create_struct_ts
    })
}

fn read_ts(ty: &impl ToTokens, lifetime: &impl ToTokens, variant: Option<&impl ToTokens>) -> TokenStream {
    match variant {
        Some(variant) => quote! { <#variant as bird_protocol::ProtocolVariantReadable<#lifetime, #ty>>::read_variant(__cursor)? },
        None => quote! { <#ty as bird_protocol::ProtocolReadable<#lifetime>>::read(__cursor)? },
    }
}