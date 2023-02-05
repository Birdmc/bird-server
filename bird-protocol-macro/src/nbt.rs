use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Field, Fields};
use syn::parse::{Parse, ParseStream};
use crate::shared::{Attributes, obligate_lifetime, parse_attributes};

pub struct NbtCompoundAttributes {
    pub transparent: (bool, Span),
}

impl Default for NbtCompoundAttributes {
    fn default() -> Self {
        Self {
            transparent: (false, Span::call_site()),
        }
    }
}

pub struct NbtCompoundTransparentFieldAttributes {
    pub transparent: (bool, Span),
}

impl Default for NbtCompoundTransparentFieldAttributes {
    fn default() -> Self {
        Self {
            transparent: (false, Span::call_site()),
        }
    }
}

#[derive(Default)]
pub struct NbtCompoundFieldAttributes {
    pub name: Option<(String, Span)>,
    pub variant: Option<TokenStream>,
}

impl Parse for NbtCompoundAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes = Attributes::parse(input)?;
        Ok(Self {
            transparent: attributes.remove_boolean_value(&"transparent".into(), false)?,
        })
    }
}

impl Parse for NbtCompoundFieldAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes = Attributes::parse(input)?;
        Ok(Self {
            name: attributes.remove_string_attribute(&"name".into())?,
            variant: attributes.remove_ts_attribute(&"variant".into())?,
        })
    }
}

pub fn impl_derive(item: proc_macro::TokenStream) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs,
        ident,
        data,
        mut generics,
        ..
    } = syn::parse(item)?;
    let (lifetime, spec_impl_generics) = obligate_lifetime(&mut generics)?;
    match data {
        Data::Struct(data_struct) => { // Compound
            let compound_attrs: NbtCompoundAttributes = parse_attributes(&attrs, "bnbt")?;
            match compound_attrs.transparent {
                (true, _span) => { unimplemented!() }
                (false, _span) => {
                    let (write_prepare, read_end, fields) = match data_struct.fields {
                        Fields::Unit => (quote! {}, quote! { Ok(Self) }, Vec::new()),
                        Fields::Unnamed(_) => return Err(syn::Error::new(Span::call_site(), "Unnamed structs are not supported")),
                        Fields::Named(named) => {
                            let idents: Vec<_> = named.named.iter().map(|field| field.ident.clone()).collect();
                            let mut fields = Vec::new();
                            for field in named.named {
                                let field_attrs: NbtCompoundFieldAttributes = parse_attributes(&field.attrs, "bnbt")?;
                                fields.push((
                                    field_attrs.variant.unwrap_or_else(|| field.ty.to_token_stream()),
                                    field_attrs.name
                                        .map(|(name, _)| name)
                                        .unwrap_or_else(|| field.ident.as_ref().unwrap().to_string()),
                                    field,
                                ))
                            }
                            (
                                quote! { #(let #idents = &self.#idents;)* },
                                quote! { Ok(Self { #(#idents,)* })  },
                                fields,
                            )
                        }
                    };
                    let (_, type_generics, where_clause) = generics.split_for_impl();
                    let (impl_generics, ..) = spec_impl_generics.split_for_impl();
                    let write = fields.iter()
                        .map(|(variant, name, field)| {
                            let Field { ident, ty, .. } = field;
                            quote! {
                                if <#variant as bird_protocol::nbt::NbtTagVariant<#lifetime, #ty>>::should_write_nbt_variant(#ident) {
                                    <u8 as bird_protocol::nbt::NbtTag<#lifetime>>::write_nbt(
                                        &<#variant as bird_protocol::nbt::NbtTagVariant<#lifetime, #ty>>::get_nbt_tag(#ident)?,
                                        __writer
                                    )?;
                                    bird_protocol::nbt::write_nbt_str(#name, __writer)?;
                                    <#variant as bird_protocol::nbt::NbtTagVariant<#lifetime, #ty>>::write_nbt_variant(#ident, __writer)?;
                                }
                            }
                        });
                    let read_prepare = fields.iter()
                        .map(|(_variant, _name, field)| {
                            let Field { ident, ty, .. } = field;
                            quote! {
                                let mut #ident: std::option::Option::<#ty> = std::option::Option::None;
                            }
                        });
                    let read_fields = fields.iter()
                        .map(|(variant, name, field)| {
                            let Field { ident, ty, .. } = field;
                            quote! {
                                #name => {
                                    if !<#variant as bird_protocol::nbt::NbtTagVariant<#lifetime, #ty>>::check_nbt_tag(__tag) {
                                        return bird_protocol::ProtocolResult::Err(bird_protocol::ProtocolError::Any(
                                            bird_protocol::anyhow::Error::msg("Bad tag")
                                        ));
                                    }
                                    #ident.replace(<#variant as bird_protocol::nbt::NbtTagVariant<#lifetime, #ty>>::read_nbt_variant(__cursor)?);
                                    bird_protocol::ProtocolResult::Ok(())
                                }
                            }
                        });
                    let read_end_prepare = fields.iter()
                        .map(|(variant, _name, field)| {
                            let Field { ident, ty, .. } = field;
                            quote! {
                                let #ident = #ident
                                    .or_else(|| <#variant as bird_protocol::nbt::NbtTagVariant<#lifetime, #ty>>::default_nbt_variant_value())
                                    .ok_or_else(|| bird_protocol::ProtocolError::Any(bird_protocol::anyhow::Error::msg("Not each tag")))?;
                            }
                        });
                    let skip_fields = fields.iter()
                        .map(|(variant, name, field)| {
                            let Field { ty, .. } = field;
                            quote! {
                                #name => {
                                    if !<#variant as bird_protocol::nbt::NbtTagVariant<#lifetime, #ty>>::check_nbt_tag(__tag) {
                                        return bird_protocol::ProtocolResult::Err(bird_protocol::ProtocolError::Any(
                                            bird_protocol::anyhow::Error::msg("Bad tag")
                                        ));
                                    }
                                    __result += <#variant as bird_protocol::nbt::NbtTagVariant<#lifetime, #ty>>::skip_nbt_variant(__cursor, 1)?;
                                }
                            }
                        });
                    Ok(quote! {
                        impl #impl_generics bird_protocol::nbt::NbtTag<#lifetime> for #ident #type_generics #where_clause {
                            const NBT_TAG: u8 = bird_protocol::nbt::NBT_TAG_COMPOUND;

                            fn write_nbt<W: bird_protocol::ProtocolWriter>(&self, __writer: &mut W) -> bird_protocol::anyhow::Result<()> {
                                #write_prepare
                                #(#write)*
                                <u8 as bird_protocol::nbt::NbtTag<#lifetime>>::write_nbt(&0, __writer)
                            }

                            fn read_nbt<C: bird_protocol::ProtocolCursor<'a>>(__cursor: &mut C) -> bird_protocol::ProtocolResult<Self> {
                                #(#read_prepare)*
                                bird_protocol::nbt::compound::read_nbt_compound(__cursor, |__tag, __name, __cursor| {
                                    match <Cow<#lifetime, str> as std::convert::AsRef<str>>::as_ref(&__name) {
                                        #(#read_fields,)*
                                        _ => bird_protocol::ProtocolResult::Err(bird_protocol::ProtocolError::Any(
                                            bird_protocol::anyhow::Error::msg("Bad name")
                                        ))
                                    }
                                })?;
                                #(#read_end_prepare)*
                                #read_end
                            }

                            fn skip_nbt<C: bird_protocol::ProtocolCursor<'a>>(__cursor: &mut C, __amount: usize) -> bird_protocol::ProtocolResult<usize> {
                                let mut __result: usize = 0;
                                for _ in 0..__amount {
                                    bird_protocol::nbt::compound::read_nbt_compound(__cursor, |__tag, __name, __cursor| {
                                        match <Cow<#lifetime, str> as std::convert::AsRef<str>>::as_ref(&__name) {
                                            #(#skip_fields,)*
                                            _ => bird_protocol::ProtocolResult::Err(bird_protocol::ProtocolError::Any(
                                                bird_protocol::anyhow::Error::msg("Bad name")
                                            ))?
                                        };
                                        __result += 3 + __name.len();
                                        bird_protocol::ProtocolResult::Ok(())
                                    })?;
                                }
                                Ok(__result)
                            }
                        }
                    })
                }
            }
        }
        Data::Enum(_data_enum) => {
            unimplemented!()
        }
        Data::Union(_) => Err(syn::Error::new(Span::call_site(), "Union type is not supported")),
    }
}