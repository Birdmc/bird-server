use convert_case::{Case, Casing};
use minecraft_data_rs::{models::item::Item, Api};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub fn generate_items(api: &Api) -> syn::Result<TokenStream> {
    let mut item_const_ts = Vec::new();
    let mut item_from_id_ts = Vec::new();
    let mut item_from_name_ts = Vec::new();
    for item in api.items.items_array().unwrap() {
        let Item {
            id,
            name,
            stack_size,
            ..
        } = item;
        let item_const_ident = Ident::new(name.to_case(Case::UpperSnake).as_str(), Span::call_site());
        item_const_ts.push(quote! { 
            pub const #item_const_ident: super::ItemData<'static> = super::ItemData::new(
                #id, #name, #stack_size
            );
        });
        item_from_id_ts.push(quote! { #id => std::option::Option::Some(&item_data:: #item_const_ident ) });
        item_from_name_ts.push(quote! { #name => std::option::Option::Some(&item_data:: #item_const_ident) });
    }
    Ok(quote! {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct ItemData<'a> { 
            pub id: u32,
            pub name: &'a str,
            pub stack_size: u8
        }

        pub mod item_data {
            #(#item_const_ts)*
        }

        impl<'a> ItemData<'a> {
            const fn new(id: u32, name: &'a str, stack_size: u8) -> Self {
                Self { id, name, stack_size }
            }

            pub const fn from_id(id: u32) -> std::option::Option<&'static Self> {
                match id {
                    #(#item_from_id_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub fn from_name(name: &str) -> std::option::Option<&'static Self> {
                match name {
                    #(#item_from_name_ts,)*
                    _ => std::option::Option::None
                }
            }
        }
    })
}
