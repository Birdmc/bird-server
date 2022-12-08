use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Fields, Type};
use crate::shared::{FieldAttributes, GhostValue, ObjectAttributes, parse_attributes, VariantAttributes};

pub fn impl_derive(item: proc_macro::TokenStream) -> syn::Result<TokenStream> {
    let item: DeriveInput = syn::parse(item)?;
    let DeriveInput {
        ident,
        generics,
        attrs,
        data,
        ..
    } = item;
    let object_attributes: ObjectAttributes = parse_attributes(&attrs, "bp")?;
    let size = match data {
        Data::Struct(data_struct) => {
            let (min, max) = fields_size(data_struct.fields, object_attributes.ghost_values.into_iter())?;
            quote! { (#min .. #max) }
        }
        Data::Enum(data_enum) => {
            let mut min_variants_size = Vec::new();
            let mut max_variants_size = Vec::new();
            for variant in data_enum.variants {
                let variant_attributes: VariantAttributes = parse_attributes(&variant.attrs, "bp")?;
                let (min_variant_size, max_variant_size) = fields_size(
                    variant.fields,
                    object_attributes.ghost_values.iter().cloned().chain(variant_attributes.ghost_values.into_iter())
                )?;
                min_variants_size.push(min_variant_size);
                max_variants_size.push(max_variant_size);
            }
            let (min_key, max_key) = enum_key_size(&object_attributes)?;
            quote! { (
                bird_protocol::__private::add_u32_without_overflow_array([
                    #min_key,
                    bird_protocol::__private::min_u32_array([#(#min_variants_size,)*]),
                ])
                ..
                bird_protocol::__private::add_u32_without_overflow_array([
                    #max_key,
                    bird_protocol::__private::max_u32_array([#(#max_variants_size,)*]),
                ])
            ) }
        }
        Data::Union(_) => return Err(syn::Error::new(Span::mixed_site(), "Union type is not supported")),
    };
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics bird_protocol::ProtocolSize for #ident #type_generics #where_clause {
            const SIZE: std::ops::Range<u32> = #size;
        }
    })
}

pub fn enum_key_size(object_attributes: &ObjectAttributes) -> syn::Result<(TokenStream, TokenStream)> {
    let key_ty = object_attributes.key_variant.as_ref()
        .or_else(|| object_attributes.key_ty.as_ref())
        .ok_or_else(|| syn::Error::new(Span::call_site(), "You must set ty or variant for key of your enum"))?;
    Ok((min_size_ts(&key_ty), max_size_ts(&key_ty)))
}

pub fn fields_size(fields: Fields, ghost_values: impl Iterator<Item=GhostValue>) -> syn::Result<(TokenStream, TokenStream)> {
    enum Size {
        Ty(TokenStream),
        Val(TokenStream),
    }
    let mut min_size_types = Vec::new();
    let mut max_size_types = Vec::new();
    let mut fields_with_attrs: Vec<(_, FieldAttributes)> = Vec::new();
    for field in fields {
        let field_attributes = parse_attributes(&field.attrs, "bp")?;
        fields_with_attrs.push((field, field_attributes));
    }
    for ty in fields_with_attrs.into_iter()
        .map(|(field, field_attributes)|
            Size::Ty(field_attributes.variant.unwrap_or_else(|| field.ty.into_token_stream()))
        )
        .chain(ghost_values.into_iter().map(|ghost_value| ghost_value.variant
            .or(ghost_value.ty)
            .map(|v| Size::Ty(v))
            .unwrap_or_else(|| {
                let value = ghost_value.value;
                Size::Val(quote! { bird_protocol::__private::size_of_val(&#value) })
            })
        )) {
        match ty {
            Size::Ty(ty) => {
                min_size_types.push(min_size_ts(&ty));
                max_size_types.push(max_size_ts(&ty));
            },
            Size::Val(val) => {
                min_size_types.push(quote! { #val.start });
                max_size_types.push(quote! { #val.end });
            }
        }
    }
    Ok((
        quote! { bird_protocol::__private::add_u32_without_overflow_array([#(#min_size_types,)*]) },
        quote! { bird_protocol::__private::add_u32_without_overflow_array([#(#max_size_types,)*]) }
    ))
}

pub fn min_size_ts(ty: &impl ToTokens) -> TokenStream {
    quote! { <#ty as bird_protocol::ProtocolSize>::SIZE.start }
}

pub fn max_size_ts(ty: &impl ToTokens) -> TokenStream {
    quote! { <#ty as bird_protocol::ProtocolSize>::SIZE.end }
}