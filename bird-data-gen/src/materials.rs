use convert_case::{Casing, Case};
use minecraft_data_rs::Api;
use proc_macro2::{TokenStream, Span, Ident};
use quote::quote;

pub fn generate_materials(api: &Api) -> syn::Result<TokenStream> {
    let mut material_enum_ts = Vec::new();
    let mut material_from_name_ts = Vec::new();
    let mut material_name_ts = Vec::new();
    let mut material_value_ts = Vec::new();
    for (name, ids) in api.materials.materials().unwrap() {
        let material_enum_ident = Ident::new(name.replace(|ch: char| ch == ';' || ch == '/', "_").to_case(Case::Pascal).as_str(), Span::call_site());
        material_from_name_ts.push(quote! { #name => std::option::Option::Some(Self:: #material_enum_ident)});
        material_name_ts.push(quote! { Self:: #material_enum_ident => #name });
        let mut material_id_to_value_ts = Vec::new();
        for (id, value) in ids {
            let id = id.parse::<i32>().unwrap();
            material_id_to_value_ts.push(quote! { #id => std::option::Option::Some(#value) });
        }
        material_value_ts.push(quote! { Self:: #material_enum_ident => match item {
            #(#material_id_to_value_ts,)*
            _ => std::option::Option::None
        }});
        material_enum_ts.push(material_enum_ident);
    }
    Ok(quote! {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Material { #(#material_enum_ts,)* }

        impl Material {
            pub fn from_name(name: &str) -> std::option::Option<Self> {
                match name {
                    #(#material_from_name_ts,)*
                    _ => std::option::Option::None
                }
            }

            pub const fn get_name(&self) -> &'static str {
                match self {
                    #(#material_name_ts,)*
                }
            }

            pub const fn get_value(&self, item: i32) -> std::option::Option<f32> {
                match self {
                    #(#material_value_ts,)*
                }
            }
        }
    })
}