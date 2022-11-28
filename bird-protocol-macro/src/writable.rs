use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Field, Fields, parse_macro_input, Variant};
use crate::shared::{create_prepared_fields, create_prepared_variants, ObjectAttributes, parse_attributes};

pub fn impl_derive(item: proc_macro::TokenStream) -> syn::Result<TokenStream> {
    let item: DeriveInput = syn::parse(item)?;
    let DeriveInput {
        ident,
        data,
        generics,
        attrs,
        ..
    } = item;
    let object_attributes: ObjectAttributes = parse_attributes(&attrs, "bp")?;
    let function_body = match data {
        Data::Struct(data_struct) => {
            let write_match = write_match(quote! { Self }, &data_struct.fields)?;
            let write_fields = write_fields(data_struct.fields)?;
            quote! { #write_match => { #write_fields }, }
        }
        Data::Enum(data_enum) => {
            let key_ty = object_attributes.key_ty.as_ref().ok_or_else(|| syn::Error::new(Span::call_site(), "You should provide key_ty for enum object"))?;
            let variants = create_prepared_variants(data_enum.variants.into_iter(), &object_attributes)?;
            let mut variant_matches = Vec::new();
            for (variant, variant_value, _variant_attributes) in variants {
                let Variant {
                    fields,
                    ident,
                    ..
                } = variant;
                let write_match = write_match(quote! { Self::#ident }, &fields)?;
                let write_fields = write_fields(fields)?;
                let write_key = write_ts(&quote! { (#variant_value as #key_ty) } , key_ty, object_attributes.key_variant.as_ref());
                variant_matches.push(quote! { #write_match => { #write_key #write_fields } });
            }
            quote! {
                #(#variant_matches,)*
                _ => unreachable!()
            }
        }
        Data::Union(_) => return Err(syn::Error::new(Span::call_site(), "Union is not supported")),
    };
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics bird_protocol::ProtocolWritable for #ident #type_generics #where_clause {
            fn write<W: bird_protocol::ProtocolWriter>(&self, __writer: &mut W) -> bird_protocol::anyhow::Result<()> {
                match self {
                    #function_body
                }
                bird_protocol::anyhow::Result::Ok(())
            }
        }
    })
}

pub fn write_match(key: impl ToTokens, fields: &Fields) -> syn::Result<TokenStream> {
    Ok(match fields {
        Fields::Unit => quote! { #key },
        Fields::Unnamed(ref unnamed) => {
            let mut idents = Vec::new();
            for counter in 0..unnamed.unnamed.len() {
                idents.push(Ident::new(format!("__{}", counter).as_str(), Span::call_site()));
            }
            quote! { #key(#(ref #idents,)*) }
        }
        Fields::Named(ref named) => {
            let mut idents = Vec::new();
            for field in &named.named {
                idents.push(field.ident.as_ref().unwrap());
            }
            quote! { #key { #(ref #idents,)* } }
        }
    })
}

pub fn write_fields(fields: Fields) -> syn::Result<TokenStream> {
    let fields = create_prepared_fields(fields)?;
    let mut writes_ts = Vec::new();
    for (field, field_attrs) in fields {
        let Field {
            ident,
            ty,
            ..
        } = field;
        let write_ts = write_ts(&ident.unwrap(), &ty, field_attrs.variant.as_ref());
        writes_ts.push(write_ts)
    }
    Ok(quote! { #(#writes_ts;)* })
}

pub fn write_ts(write: &impl ToTokens, ty: &impl ToTokens, variant: Option<&impl ToTokens>) -> TokenStream {
    match variant {
        Some(variant) => quote! { <#variant as bird_protocol::ProtocolVariantWritable<#ty>>::write_variant(&#write, __writer)? },
        None => quote! { <#ty as bird_protocol::ProtocolWritable>::write(&#write, __writer)? },
    }
}