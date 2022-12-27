use std::collections::HashSet;

use convert_case::{Case, Casing};
use minecraft_data_rs::{models::biome::Biome, Api};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub fn generate_biomes(api: &Api) -> syn::Result<TokenStream> {
    let mut categories = HashSet::new();
    let mut precipitations = HashSet::new();
    let mut biome_from_id_ts = Vec::new();
    let mut biome_from_name_ts = Vec::new();
    let mut biome_consts = Vec::new();
    for biome in api.biomes.biomes_array().unwrap() {
        let Biome {
            id,
            name,
            category,
            temperature,
            precipitation,
            dimension,
            color,
            rainfall,
            display_name: _display_name,
            depth: _depth,
        } = biome;
        let biome_const_ident = Ident::new(name.to_case(Case::UpperSnake).as_str(), Span::call_site());
        let dimension_enum_ident = Ident::new(dimension.to_case(Case::Pascal).as_str(), Span::call_site());
        let category_enum_ident = Ident::new(category.to_case(Case::Pascal).as_str(), Span::call_site());
        let precipitation_enum_ident = Ident::new(precipitation.to_case(Case::Pascal).as_str(), Span::call_site());
        biome_from_id_ts.push(quote! { #id => std::option::Option::Some(&biome_data:: #biome_const_ident)});
        biome_from_name_ts.push(quote! { #name => std::option::Option::Some(&biome_data:: #biome_const_ident)});
        biome_consts.push(quote! { 
            pub const #biome_const_ident: super::BiomeData<'static> = super::BiomeData::new(
                    #id, #name, super::BiomeCategory:: #category_enum_ident,
                    #temperature, super::BiomePrecipitation:: #precipitation_enum_ident, super::WorldDimension:: #dimension_enum_ident,
                    #color, #rainfall 
            ); 
        });
        categories.insert(category_enum_ident);
        precipitations.insert(precipitation_enum_ident);
    }
    let categories = categories.into_iter().collect::<Vec<Ident>>();
    let precipitations = precipitations.into_iter().collect::<Vec<Ident>>();
    let register_count = biome_consts.len();
    Ok(quote! {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum BiomeCategory { #(#categories,)* }

        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum BiomePrecipitation { #(#precipitations,)* }

        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct BiomeData<'a> {
            pub id: u32,
            pub name: &'a str,
            pub category: BiomeCategory,
            pub temperature: f32,
            pub precipitation: BiomePrecipitation,
            pub dimension: WorldDimension,
            pub color: u32,
            pub rain_fall: f32,
        }

        pub const BIOME_COUNT: usize = #register_count;

        pub mod biome_data {
            #(#biome_consts)*
        }
        
        impl<'a> BiomeData<'a> {

            const fn new(
                id: u32, name: &'a str, category: BiomeCategory, 
                temperature: f32, precipitation: BiomePrecipitation, dimension: WorldDimension,
                color: u32, rain_fall: f32
            ) -> Self {
                Self { id, name, category, temperature, precipitation, dimension, color, rain_fall }
            } 

            pub const fn from_id(id: u32) -> std::option::Option<&'static Self> {
                match id { 
                    #(#biome_from_id_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub fn from_name(name: &str) -> std::option::Option<&'static Self> {
                match name {
                    #(#biome_from_name_ts,)*
                    _ => std::option::Option::None
                }
            }
        }
    })
}
