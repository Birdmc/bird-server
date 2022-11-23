use std::collections::HashMap;
use std::str::FromStr;
use proc_macro2::{Ident, Span};
use syn::{Expr, ExprAssign, ExprPath, Field, Fields, GenericParam, Generics, Lifetime, LifetimeDef, Lit, Token};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

pub struct ObjectAttributes {}

pub struct VariantAttributes {}

#[derive(Default)]
pub struct FieldAttributes {
    pub order: Option<(u32, Span)>,
}

pub struct Attributes {
    pub expressions: HashMap<String, Expr>,
    pub span: Span,
}

impl Attributes {
    pub fn remove_attribute(&mut self, name: &String) -> Option<Expr> {
        self.expressions.remove(name)
    }

    pub fn remove_string_attribute(&mut self, name: &String) -> syn::Result<Option<(String, Span)>> {
        match self.remove_attribute(name) {
            Some(expr) => {
                let span = expr.span();
                expr_into_string(expr).map(|str| Some((str, span)))
            }
            None => Ok(None),
        }
    }

    pub fn remove_str_parse_attribute<T>(&mut self, name: &String) -> syn::Result<Option<(T, Span)>>
        where T: FromStr, <T as FromStr>::Err: std::fmt::Display {
        match self.remove_attribute(name) {
            Some(expr) => {
                let expr_span = expr.span();
                match expr {
                    Expr::Lit(expr_lit) => match expr_lit.lit {
                        Lit::Str(lit_str) => lit_str.value().parse().map_err(|_| ()),
                        Lit::Int(lit_int) => lit_int.base10_parse().map_err(|_| ()),
                        _ => Err(())
                    },
                    _ => Err(())
                }
                    .map(|value| Some((value, expr_span)))
                    .map_err(|_| syn::Error::new(expr_span, format!("Should be literal that is possible to convert into {}", std::any::type_name::<T>()).as_str()))
            },
            None => Ok(None),
        }
    }
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let list: Punctuated<ExprAssign, Token![,]> = parse_punctuated(&input)?;
        let mut expressions = HashMap::new();
        for expr_assign in list {
            let left = *expr_assign.left;
            let left_span = left.span();
            let key = expr_into_string(left)?;
            if let Some(_) = expressions.insert(key, *expr_assign.right) {
                return Err(syn::Error::new(left_span, "This key already used"));
            }
        }
        Ok(Self {
            expressions,
            span: input.span(),
        })
    }
}

impl Parse for FieldAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes: Attributes = input.parse()?;
        Ok(Self {
            order: attributes.remove_str_parse_attribute(&"order".into())?,
        })
    }
}

fn expr_into_string(expr: Expr) -> syn::Result<String> {
    match expr {
        Expr::Path(ref path) => Ok(expr_path_into_string(path)),
        Expr::Lit(ref lit) => match lit.lit {
            Lit::Str(ref lit_str) => Ok(lit_str.value()),
            _ => Err(()),
        },
        _ => Err(())
    }.map_err(|_| syn::Error::new(expr.span(), "Expected ident or string"))
}

fn expr_path_into_string(path: &ExprPath) -> String {
    path.path.segments.iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<String>>()
        .join("::")
}

fn parse_punctuated<T: Parse, P: Parse + Default>(input: &ParseStream) -> syn::Result<Punctuated<T, P>> {
    let mut result = Punctuated::new();
    while !input.is_empty() {
        result.push(input.parse()?);
        if input.is_empty() {
            break;
        }
        result.push_punct(input.parse()?);
    }
    Ok(result)
}

fn parse_attributes<A: Parse + Default>(attrs: &Vec<syn::Attribute>, attr_name: &str) -> syn::Result<A> {
    attrs.iter()
        .find(|attr| attr.path.is_ident(attr_name))
        .map(|attr| attr.parse_args())
        .unwrap_or_else(|| Ok(A::default()))
}

pub fn create_prepared_fields(fields: Fields) -> syn::Result<Vec<(Field, FieldAttributes)>> {
    let mut counter = 0;
    let mut ordered_fields = Vec::new();
    let mut specific_ordered_fields = HashMap::new();
    for mut field in fields {
        if None == field.ident {
            field.ident.replace(Ident::new(format!("__{}", counter).as_str(), Span::call_site()));
            counter += 1;
        }
        let field_attributes: FieldAttributes = parse_attributes(&field.attrs, "bp")?;
        match field_attributes.order {
            Some((order, span)) => if let Some(_) = specific_ordered_fields.insert(order, (field, field_attributes)) {
                return Err(syn::Error::new(span, "Repeated order value"))
            },
            None => ordered_fields.push((field, field_attributes)),
        }
    }
    let mut specific_ordered_fields: Vec<(u32, (Field, FieldAttributes))> = specific_ordered_fields.into_iter().collect();
    specific_ordered_fields.sort_by(|(first, _), (second, _)| first.cmp(second));
    for (order, obj) in specific_ordered_fields {
        ordered_fields.insert(order as usize, obj);
    }
    Ok(ordered_fields)
}

pub fn obligate_lifetime(generics: &mut Generics) -> syn::Result<(LifetimeDef, Generics)> {
    let mut lifetimes = generics.lifetimes();
    match lifetimes.next() {
        Some(lifetime_def) => match lifetimes.next() {
            None => Ok((lifetime_def.clone(), generics.clone())),
            Some(bad_lifetime_def) => Err(syn::Error::new(bad_lifetime_def.span(), "Two or more lifetimes are not supported")),
        },
        None => {
            drop(lifetimes);
            let mut generics = generics.clone();
            let lifetime_def = LifetimeDef::new(Lifetime::new("'a", Span::call_site()));
            generics.params.insert(0, GenericParam::Lifetime(lifetime_def));
            Ok(match generics.params.first().unwrap() {
                GenericParam::Lifetime(lifetime_def) => (lifetime_def.clone(), generics),
                _ => unreachable!(),
            })
        }
    }
}