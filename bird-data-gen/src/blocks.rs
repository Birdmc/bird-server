//! Generates enum of all vanilla blocks with methods (and some helper enums):
//! - const from_id(id: u32) -> Option<Self> -- will return block with default state
//! - fn from_name(name: &str) -> Option<Self> -- will return block with default state
//! - fn from_id_with_state(id: u32, state: &HashMap<&str, &str>) -> Option<Self> -- should be changed to result
//! - fn from_name_with_state(name: &str, state: &HashMap<&str, &str>) -> Option<Self> -- should be changed to result
//! - const from_state(state: u32) -> Option<Self>
//! - const get_id(&self) -> u32
//! - const get_name(&self) -> &'static str
//! - const get_hardness(&self) -> f32
//! - const get_blast_resistance(&self) -> f32
//! - const is_diggable(&self) -> bool
//! - fn get_material(&self) -> Material
//! - const is_transparent(&self) -> bool
//! - const get_emit_light(&self) -> u8
//! - const get_filter_light(&self) -> u8
//! - const get_state(&self) -> u32
//! - const get_drops(&self) -> &'static [u32] -- will return slice of items ids
//! - fn get_item(&self) -> Option<Item>

use std::collections::HashMap;

use convert_case::{Case, Casing};
use minecraft_data_rs::{
    models::block::{Block, StateType},
    Api,
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub fn generate_blocks(api: &Api) -> syn::Result<TokenStream> {

    let mut blocks_enum_ts = Vec::new();
    let mut blocks_from_id_ts = Vec::new();
    let mut blocks_from_name_ts = Vec::new();
    // let mut blocks_from_id_with_state_ts = Vec::new();
    // let mut blocks_from_name_with_state_ts = Vec::new();
    let mut blocks_from_state_ts = Vec::new();
    let mut blocks_id_ts = Vec::new();
    let mut blocks_name_ts = Vec::new();
    let mut blocks_hardness_ts = Vec::new();
    let mut blocks_resistance_ts = Vec::new();
    let mut blocks_diggable_ts = Vec::new();
    let mut blocks_material_ts = Vec::new();
    let mut blocks_transparent_ts = Vec::new();
    let mut blocks_emit_light_ts = Vec::new();
    let mut blocks_filter_light_ts = Vec::new();
    let mut blocks_state_ts = Vec::new();
    let mut blocks_drops_ts = Vec::new();
    let mut blocks_state_enums_ts = Vec::new();

    let blocks_array = api.blocks.blocks_array().unwrap();
    let mut enum_states_keyed = HashMap::new();
    for block in &blocks_array {
        for state in block.states.as_ref().unwrap() {
            if let StateType::Enum = state.state_type.clone() {
                enum_states_keyed.entry(state.name.clone())
                    .or_insert_with(|| HashMap::new())
                    .entry(state.num_values)
                    .or_insert_with(|| HashMap::new())
                    .entry(state.values.clone().unwrap())
                    .or_insert_with(|| Vec::new())
                    .push(block.name.clone())
            }
        }
    }

    let mut blocks_enum_states = HashMap::new();

    for (state_name, states) in &enum_states_keyed {
        let count_in_name = states.len() > 1;
        for (state_values_count, states) in states {
            let values_in_name = states.len() > 1;
            for (state_values, blocks) in states {
                let state_name_pascal = state_name.to_case(Case::Pascal);
                let enum_state_ident = Ident::new(match count_in_name || values_in_name {
                    true => {
                        let mut result = state_name_pascal.clone();
                        if count_in_name {
                            result.push_str(format!("{}", state_values_count).as_str())
                        }
                        if values_in_name {
                            for state_value in state_values {
                                result.push_str(state_value.as_str()[0..1].to_uppercase().as_str());
                            }
                        }
                        result
                    },
                    false => state_name_pascal
                }.as_str(), Span::call_site());
                let state_values = state_values.iter()
                    .map(|state_value| Ident::new(state_value.to_case(Case::Pascal).as_str(), Span::call_site()))
                    .collect::<Vec<Ident>>();
                blocks_state_enums_ts.push(quote! { 
                    enum #enum_state_ident {
                        #(#state_values,)*
                    }
                });
                for block in blocks {
                    blocks_enum_states.insert((block, state_name), enum_state_ident.clone()); 
                }
            }
        }
    }

    for block in blocks_array {
        let Block {
            id,
            name,
            hardness,
            blast_resistance,
            diggable,
            material,
            transparent,
            emit_light,
            filter_light,
            default_state,
            states,
            drops,
            min_state_id,
            max_state_id,
            ..
        } = block;
        let material = material.expect("material is none");
        let hardness = hardness.expect("hardness is none");
        let blast_resistance = blast_resistance.expect("resistance is none");
        let block_enum_ident = Ident::new(name.to_case(Case::Pascal).as_str(), Span::call_site());
        let min_state_id = min_state_id.expect("min state id is none") as usize;
        let max_state_id = max_state_id.expect("max state id is none") as usize;
        let default_state_id = default_state.expect("default state id is none") as usize;

        let states = states.filter(|states| !states.is_empty());

        let (default_creator, creators, block_enum_repr, block_enum_in_match_repr) = match states {
            Some(states) => {
                let state_ts = states.iter()
                    .map(|state| (state, blocks_enum_states.get(&(&name, &state.name))))
                    .map(|(state, state_ty)| (
                        Ident::new(match state.name.as_str() {
                            "type" => "ty",
                            others => others,
                        }.to_case(Case::Pascal).as_str(), Span::call_site()),
                        match state.state_type {
                            StateType::Bool => quote! { bool },
                            StateType::Enum => quote! { #state_ty },
                            StateType::Int => quote! { i32 },
                        },
                        match state.state_type {
                            StateType::Bool => vec![quote! { true }, quote! { false }],
                            StateType::Enum => state.values.as_ref().expect("statetype is enum but values is none")
                                .iter()
                                .map(|value| Ident::new(value.to_case(Case::Pascal).as_str(), Span::call_site()))
                                .map(|ident| {
                                    quote! { #state_ty :: #ident }
                                })
                                .collect(),
                            StateType::Int => state.values.as_ref().expect("statetype is int but values is none")
                                .iter()
                                .map(|value| value.parse().unwrap())
                                .map(|value: i32| quote! { #value })
                                .collect(),
                        }
                    ))
                    .collect::<Vec<(Ident, TokenStream, Vec<TokenStream>)>>();
                let mut block_enum_repr = Vec::new();
                for (state_ident, state_ty, _) in &state_ts {
                    block_enum_repr.push(quote! { #state_ident : #state_ty });
                }
                let block_enum_repr = quote! { #block_enum_ident { #(#block_enum_repr,)* } };
                let mut creators = Vec::new();
                creators.resize(max_state_id - min_state_id + 1, Vec::new());
                let mut out_repeat = 1;
                let mut in_repeat = creators.len();
                for (state_ident, _, state_values) in &state_ts {
                    let mut i = 0;
                    in_repeat /= state_values.len();
                    for _ in 0..out_repeat {
                        for state_value in state_values {
                            for _ in 0..in_repeat {
                                creators.get_mut(i).expect("creators[i] is none").push(quote! { #state_ident : #state_value });
                                i += 1;
                            }
                        }
                    }
                    debug_assert!(i == creators.len());
                    out_repeat *= state_values.len();
                }
                debug_assert!(out_repeat == max_state_id - min_state_id + 1);
                debug_assert!(in_repeat == 1);
                let creators = creators.into_iter()
                    .map(|creator| quote! { Self:: #block_enum_ident {#(#creator,)*} })
                    .collect::<Vec<TokenStream>>();
                (creators.get(default_state_id - min_state_id).unwrap().clone(), creators, block_enum_repr, quote! { Self:: #block_enum_ident {..}})
            },
            None => {
                let default_creator = quote!{ Self:: #block_enum_ident };
                (default_creator.clone(), vec![default_creator.clone()], quote! { #block_enum_ident }, default_creator)
            }
        };

        blocks_from_id_ts.push(quote! { #id => std::option::Option::Some(#default_creator) });
        blocks_from_name_ts.push(quote! { #name => std::option::Option::Some(#default_creator) });
        { 
            let mut current_state = min_state_id as u32;
            for creator in &creators {
                blocks_from_state_ts.push(quote! { #current_state => std::option::Option::Some(#creator) });
                blocks_state_ts.push(quote!{ #creator => #current_state });
                current_state += 1;
            }
        }
        blocks_id_ts.push(quote! { #block_enum_in_match_repr => #id });
        blocks_name_ts.push(quote! { #block_enum_in_match_repr => #name });
        blocks_hardness_ts.push(quote! { #block_enum_in_match_repr => #hardness});
        blocks_resistance_ts.push(quote! { #block_enum_in_match_repr => #blast_resistance});
        blocks_diggable_ts.push(quote! { #block_enum_in_match_repr => #diggable});
        blocks_material_ts.push(quote! { #block_enum_in_match_repr => Material::from_name( #material )});
        blocks_transparent_ts.push(quote! { #block_enum_in_match_repr => #transparent});
        blocks_emit_light_ts.push(quote! { #block_enum_in_match_repr => #emit_light});
        blocks_filter_light_ts.push(quote! { #block_enum_in_match_repr => #filter_light});
        blocks_drops_ts.push(quote! { #block_enum_in_match_repr => &[#(#drops,)*]});
        blocks_enum_ts.push(block_enum_repr);
    }

    Ok(quote! {

        #(
            #[derive(Clone, Copy, Debug, PartialEq)]
            pub #blocks_state_enums_ts
        )*

        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Block { #(#blocks_enum_ts,)* }

        impl Block {
            pub const fn from_id(id: u32) -> std::option::Option<Self> {
                match id {
                    #(#blocks_from_id_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub fn from_name(name: &str) -> std::option::Option<Self> {
                match name {
                    #(#blocks_from_name_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub const fn from_state(state: u32) -> std::option::Option<Self> {
                match state {
                    #(#blocks_from_state_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub const fn get_id(&self) -> u32 {
                match self {
                    #(#blocks_id_ts,)*
                }
            }

            pub const fn get_name(&self) -> &'static str {
                match self {
                    #(#blocks_name_ts,)*
                }
            }

            pub const fn get_hardness(&self) -> f32 {
                match self {
                    #(#blocks_hardness_ts,)*
                }
            }

            pub const fn get_blast_resistance(&self) -> f32 {
                match self {
                    #(#blocks_resistance_ts,)*
                }
            }

            pub const fn is_diggable(&self) -> bool {
                match self {
                    #(#blocks_diggable_ts,)*
                }
            }

            pub fn get_material(&self) -> std::option::Option<Material> {
                match self {
                    #(#blocks_material_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub const fn is_transparent(&self) -> bool {
                match self {
                    #(#blocks_transparent_ts,)*
                }
            }

            pub const fn get_emit_light(&self) -> u8 {
                match self {
                    #(#blocks_emit_light_ts,)*
                }
            }

            pub const fn get_filter_light(&self) -> u8 {
                match self {
                    #(#blocks_filter_light_ts,)*
                }
            }

            pub const fn get_drops(&self) -> &'static [u32] {
                match self {
                    #(#blocks_drops_ts,)*
                }
            }

            pub fn get_item(&self) -> std::option::Option<Item> {
                Item::from_name(self.get_name())
            }
        }
    })
}
