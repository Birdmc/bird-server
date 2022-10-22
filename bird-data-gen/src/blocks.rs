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
    let mut blocks_state_ts = Vec::new();
    let mut blocks_state_enums_ts = Vec::new();
    let mut blocks_data_from_id_ts = Vec::new();
    let mut blocks_data_from_name_ts = Vec::new();
    let mut blocks_data_ts = Vec::new();
    let mut blocks_const_data_ts = Vec::new();

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
                        }.to_case(Case::Snake).as_str(), Span::call_site()),
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
                blocks_state_ts.push(quote!{ #creator => std::option::Option::Some(#current_state) });
                current_state += 1;
            }
        }
        let block_data_const_ident = Ident::new(name.to_case(Case::UpperSnake).as_str(), Span::call_site());
        blocks_const_data_ts.push(quote! { 
            pub const #block_data_const_ident: super::BlockData<'static> = super::BlockData::new(
                #id, #name, #hardness, #blast_resistance, #diggable, #material, #transparent, #emit_light, #filter_light, &[#(#drops,)*]
            );
        });
        blocks_data_ts.push(quote! { #block_enum_in_match_repr => &block_data:: #block_data_const_ident });
        blocks_data_from_id_ts.push(quote! { #id => std::option::Option::Some(&block_data:: #block_data_const_ident ) });
        blocks_data_from_name_ts.push(quote! { #name => std::option::Option::Some(&block_data:: #block_data_const_ident ) });
        blocks_enum_ts.push(block_enum_repr);
    }

    Ok(quote! {

        #(
            #[derive(Clone, Copy, Debug, PartialEq)]
            pub #blocks_state_enums_ts
        )*

        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct BlockData<'a> {
            pub id: u32,
            pub name: &'a str,
            pub hardness: f32,
            pub blast_resistance: f32,
            pub diggable: bool,
            pub material: &'a str,
            pub transparent: bool,
            pub emit_light: u8,
            pub filter_light: u8,
            pub drops: &'a [u32],
        }

        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Block { #(#blocks_enum_ts,)* }

        mod block_data {
            #(#blocks_const_data_ts)*
        }

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

            pub const fn get_data(&self) -> &'static BlockData<'static> {
                match self {
                    #(#blocks_data_ts,)*
                }
            }

            pub const fn get_state(&self) -> std::option::Option<u32> {
                match self {
                    #(#blocks_state_ts,)*
                    _ => std::option::Option::None
                }
            }
        }

        impl<'a> BlockData<'a> {
            const fn new(
                id: u32, name: &'a str, hardness: f32, 
                blast_resistance: f32, diggable: bool, material: &'a str,
                transparent: bool, emit_light: u8, filter_light: u8,
                drops: &'a [u32]
            ) -> Self {
                Self { 
                    id, name, hardness, blast_resistance, diggable, 
                    material, transparent, emit_light, filter_light, drops 
                }
            }

            pub const fn from_id(id: u32) -> std::option::Option<&'static Self> {
                match id {
                    #(#blocks_data_from_id_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub fn from_name(name: &str) -> std::option::Option<&'static Self> {
                match name {
                    #(#blocks_data_from_name_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub fn get_material(&self) -> std::option::Option<Material> {
                Material::from_name(self.material)
            } 

            pub fn as_item_data(&self) -> std::option::Option<&'static ItemData> {
                ItemData::from_name(self.name)
            }
        }
    })
}
