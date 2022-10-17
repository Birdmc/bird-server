//! Generate enum of all vanilla biomes with methods (and some helper enums):
//! - const from_id(u32) -> Option<Self>
//! - const from_name(&str) -> Option<Self>
//! - const get_id(&self) -> u32
//! - const get_name(&self) -> &'static str
//! - const get_category(&self) -> BiomeCategory
//! - const get_temperature(&self) -> f32
//! - const get_precipitation(&self) -> BiomePrecipitation
//! - const get_dimension(&self) -> WorldDimension
//! - const get_color(&self) -> u32
//! - const get_rain_fall(&self) -> f32

use std::collections::HashSet;

use convert_case::{Case, Casing};
use minecraft_data_rs::{models::biome::Biome, Api};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub fn generate_biomes(api: &Api) -> syn::Result<TokenStream> {
    let mut categories = HashSet::new();
    let mut precipitations = HashSet::new();
    let mut biome_enum_ts = Vec::new();
    let mut biome_from_id_ts = Vec::new();
    let mut biome_from_name_ts = Vec::new();
    let mut biome_id_ts = Vec::new();
    let mut biome_name_ts = Vec::new();
    let mut biome_category_ts = Vec::new();
    let mut biome_temperature_ts = Vec::new();
    let mut biome_precipitation_ts = Vec::new();
    let mut biome_dimension_ts = Vec::new();
    let mut biome_color_ts = Vec::new();
    let mut biome_rain_fall_ts = Vec::new();
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
        let biome_enum_ident = Ident::new(name.to_case(Case::Pascal).as_str(), Span::call_site());
        let dimension_enum_ident = Ident::new(dimension.to_case(Case::Pascal).as_str(), Span::call_site());
        let category_enum_ident = Ident::new(category.to_case(Case::Pascal).as_str(), Span::call_site());
        let precipitation_enum_ident = Ident::new(precipitation.to_case(Case::Pascal).as_str(), Span::call_site());
        biome_enum_ts.push(quote! { #biome_enum_ident });
        biome_from_id_ts.push(quote! { #id => std::option::Option::Some( Self:: #biome_enum_ident)});
        biome_from_name_ts.push(quote! { #name => std::option::Option::Some( Self:: #biome_enum_ident)});
        biome_id_ts.push(quote! { Self:: #biome_enum_ident => #id });
        biome_name_ts.push(quote! { Self:: #biome_enum_ident => &#name });
        biome_category_ts.push(quote! { Self:: #biome_enum_ident => BiomeCategory:: #category_enum_ident });
        biome_temperature_ts.push(quote! { Self:: #biome_enum_ident => #temperature });
        biome_precipitation_ts.push(quote! { Self:: #biome_enum_ident => BiomePrecipitation:: #precipitation_enum_ident });
        biome_dimension_ts.push(quote! { Self:: #biome_enum_ident => WorldDimension:: #dimension_enum_ident });
        biome_color_ts.push(quote! { Self:: #biome_enum_ident => #color });
        biome_rain_fall_ts.push(quote! { Self:: #biome_enum_ident => #rainfall });
        categories.insert(category_enum_ident);
        precipitations.insert(precipitation_enum_ident);
    }
    let categories = categories.into_iter().collect::<Vec<Ident>>();
    let precipitations = precipitations.into_iter().collect::<Vec<Ident>>();
    Ok(quote! {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum BiomeCategory { #(#categories,)* }

        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum BiomePrecipitation { #(#precipitations,)* }
        
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Biome { #(#biome_enum_ts,)* }
        
        impl Biome {
            pub const fn from_id(id: u32) -> std::option::Option<Self> {
                match id { 
                    #(#biome_from_id_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub fn from_name(name: &str) -> std::option::Option<Self> {
                match name {
                    #(#biome_from_name_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub const fn get_id(&self) -> u32 {
                match self {
                    #(#biome_id_ts,)*
                }
            }

            pub const fn get_name(&self) -> &'static str {
                match self {
                    #(#biome_name_ts,)*
                }
            }

            pub const fn get_category(&self) -> BiomeCategory {
                match self {
                    #(#biome_category_ts,)*
                }
            }

            pub const fn get_temperature(&self) -> f32 {
                match self { 
                    #(#biome_temperature_ts,)*
                }
            }

            pub const fn get_precipitation(&self) -> BiomePrecipitation {
                match self {
                    #(#biome_precipitation_ts,)*
                }
            }

            pub const fn get_dimension(&self) -> WorldDimension {
                match self {
                    #(#biome_dimension_ts,)*
                }
            }

            pub const fn get_color(&self) -> u32 {
                match self {
                    #(#biome_color_ts,)*
                }
            }

            pub const fn get_rain_fall(&self) -> f32 {
                match self {
                    #(#biome_rain_fall_ts,)*
                }
            }
        }
    })
}
