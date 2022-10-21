//! Generate enum of all vanilla items with methods:
//! - const from_id(u32) -> Option<Self>
//! - fn from_name(&str) -> Option<Self>
//! - const get_id(&self) -> u32
//! - const get_name(&self) -> &'static str
//! - const get_stack_size(&self) -> u8

use convert_case::{Case, Casing};
use minecraft_data_rs::{models::item::Item, Api};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub fn generate_items(api: &Api) -> syn::Result<TokenStream> {
    let mut item_enum_ts = Vec::new();
    let mut item_from_id_ts = Vec::new();
    let mut item_from_name_ts = Vec::new();
    let mut item_id_ts = Vec::new();
    let mut item_name_ts = Vec::new();
    let mut item_stack_size_ts = Vec::new();
    for item in api.items.items_array().unwrap() {
        let Item {
            id,
            name,
            stack_size,
            ..
        } = item;
        let item_enum_ident = Ident::new(name.to_case(Case::Pascal).as_str(), Span::call_site());
        item_from_id_ts.push(quote! { #id => std::option::Option::Some(Self:: #item_enum_ident ) });
        item_from_name_ts.push(quote! { #name => std::option::Option::Some(Self:: #item_enum_ident) });
        item_id_ts.push(quote! { Self:: #item_enum_ident => #id });
        item_name_ts.push(quote! { Self:: #item_enum_ident => #name });
        item_stack_size_ts.push(quote! { Self:: #item_enum_ident => #stack_size });
        item_enum_ts.push(item_enum_ident);
    }
    Ok(quote! {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Item { #(#item_enum_ts,)* }

        impl Item {
            pub const fn from_id(id: u32) -> std::option::Option<Self> {
                match id {
                    #(#item_from_id_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub fn from_name(name: &str) -> std::option::Option<Self> {
                match name {
                    #(#item_from_name_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub const fn get_id(&self) -> u32 {
                match self {
                    #(#item_id_ts,)*
                }
            }

            pub const fn get_name(&self) -> &'static str {
                match self {
                    #(#item_name_ts,)*
                }
            }

            pub const fn get_stack_size(&self) -> u8 {
                match self {
                    #(#item_stack_size_ts,)*
                }
            }
        }
    })
}
