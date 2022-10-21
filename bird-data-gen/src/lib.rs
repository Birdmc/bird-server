use biomes::generate_biomes;
use blocks::generate_blocks;
use items::generate_items;
use materials::generate_materials;
use minecraft_data_rs::{api::versions_by_minecraft_version, Api};
use proc_macro::TokenTree;
use proc_macro2::Span;
use quote::quote;

mod biomes;
mod items;
mod materials;
mod blocks;

#[proc_macro]
pub fn generate_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    generate_data_impl(input).unwrap_or_else(|e| e.into_compile_error()).into()
}

fn generate_data_impl(input: proc_macro::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let version = input.into_iter()
        .next()
        .and_then(|tt| match tt {
            TokenTree::Literal(lit) => Some(lit),
            _ => None
        })
        .ok_or_else(|| syn::Error::new(Span::call_site(), "Input should be string literal"))?;
    let version_str = version.to_string();
    let mut versions = versions_by_minecraft_version().unwrap();
    let version = versions
        .remove(&version_str[1..version_str.len()-1].to_owned())
        .ok_or_else(|| syn::Error::new(Span::call_site(), format!("Unknown version {}", version_str).as_str()))?;
    let api = Api::new(version);
    let mut result = Vec::new();
    result.push(generate_biomes(&api)?);
    result.push(generate_items(&api)?);
    result.push(generate_materials(&api)?);
    // let blocks = generate_blocks(&api)?;
    // println!("{}", blocks);
    result.push(generate_blocks(&api)?);
    Ok(quote! { #(#result)* })
}