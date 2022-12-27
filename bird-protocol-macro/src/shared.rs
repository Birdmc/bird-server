use std::collections::HashMap;
use std::str::FromStr;
use either::Either;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{Expr, ExprPath, ExprTuple, Fields, GenericParam, Generics, Lifetime, LifetimeDef, Lit, Token, Variant};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

pub struct ObjectAttributes {
    pub key_variant: Option<TokenStream>,
    pub key_ty: Option<TokenStream>,
    pub key_increment: Option<TokenStream>,
    pub key_reverse: (bool, Span),
    pub packet_id: Option<TokenStream>,
    pub packet_bound: Option<TokenStream>,
    pub packet_state: Option<TokenStream>,
    pub ghost_values: Vec<GhostValue>,
}

impl Default for ObjectAttributes {
    fn default() -> Self {
        Self {
            key_variant: None,
            key_ty: None,
            key_increment: None,
            key_reverse: (false, Span::call_site()),
            packet_id: None,
            packet_bound: None,
            packet_state: None,
            ghost_values: vec![]
        }
    }
}

#[derive(Default)]
pub struct VariantAttributes {
    pub key_value: Option<TokenStream>,
    pub ghost_values: Vec<GhostValue>,
}

#[derive(Clone)]
pub struct GhostValue {
    pub order: GhostValueOrder,
    pub value: TokenStream,
    pub ty: Option<TokenStream>,
    pub variant: Option<TokenStream>,
}

#[derive(Clone)]
pub enum GhostValueOrder {
    Begin,
    End,
    Order(u32, Span),
}

#[derive(Default)]
pub struct FieldAttributes {
    pub order: Option<(u32, Span)>,
    pub variant: Option<TokenStream>,
}

pub struct Attributes {
    pub expressions: HashMap<String, Expr>,
    pub span: Span,
}

struct AttributeAssign {
    pub key: Ident,
    pub value: Expr,
}

impl Attributes {
    pub fn remove_attribute(&mut self, name: &String) -> Option<Expr> {
        self.expressions.remove(name)
    }

    #[allow(dead_code)]
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
            }
            None => Ok(None),
        }
    }

    pub fn remove_ts_attribute(&mut self, name: &String) -> syn::Result<Option<TokenStream>> {
        match self.remove_attribute(name) {
            Some(Expr::Lit(expr_lit)) => match expr_lit.lit {
                Lit::Str(ref lit_str) => match TokenStream::from_str(&lit_str.value()) {
                    Ok(ts) => Ok(Some(ts)),
                    Err(err) => Err(syn::Error::new(expr_lit.span(), err.to_string().as_str()))
                },
                _ => Ok(Some(expr_lit.to_token_stream()))
            }
            Some(expr) => Ok(Some(expr.to_token_stream())),
            None => Ok(None),
        }
    }

    pub fn remove_ghost_values(&mut self, name: &String) -> syn::Result<Vec<GhostValue>> {
        struct GhostValuesParse(Punctuated<GhostValue, Token![,]>);

        impl Parse for GhostValuesParse {
            fn parse(input: ParseStream) -> syn::Result<Self> {
                let mut punctuated = Punctuated::new();
                while !input.is_empty() {
                    let expr_tuple: ExprTuple = input.parse()?;
                    punctuated.push(syn::parse2(expr_tuple.elems.into_token_stream())?);
                    if input.is_empty() {
                        break;
                    }
                    punctuated.push_punct(input.parse()?);
                }
                Ok(Self(punctuated))
            }
        }

        match self.remove_attribute(name) {
            Some(Expr::Array(expr_tuple)) => {
                let ghost_values: GhostValuesParse = syn::parse2(expr_tuple.elems.into_token_stream())?;
                Ok(ghost_values.0.into_iter().collect())
            },
            Some(it) => Err(syn::Error::new(it.span(), format!("Must be array of tuples"))),
            None => Ok(Vec::new()),
        }
    }

    pub fn remove_boolean_value(&mut self, name: &String, default_value: bool) -> syn::Result<(bool, Span)> {
        let attr = self.remove_attribute(name);
        match attr {
            Some(Expr::Lit(ref lit)) => match lit.lit {
                Lit::Bool(ref lit_bool) => Ok((lit_bool.value, lit_bool.span)),
                _ => Err(()),
            },
            Some(_) => Err(()),
            None => Ok((default_value, Span::call_site())),
        }.map_err(|_| syn::Error::new(attr.unwrap().span(), "Must be boolean"))
    }
}


impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        type PunctuatedList = Punctuated<AttributeAssign, Token![,]>;
        type AssignKey = Option<Ident>;
        type AssignValue = Option<Either<Expr, TokenStream>>;

        fn insert_tt_into_ts<T: ToTokens>(ts: T, input: ParseStream) -> syn::Result<TokenStream> {
            let mut depth = 0;
            input.step(|cursor| {
                let mut cursor = *cursor;
                let mut res = Vec::new();
                while let Some((tt, next)) = cursor.token_tree() {
                    match tt {
                        TokenTree::Punct(ref punct) => match punct.as_char() {
                            '<' | '(' | '{' | '[' => depth += 1,
                            '>' | ')' | '}' | ']' => depth -= 1,
                            ',' => if depth <= 0 { break; }
                            _ => {}
                        },
                        _ => {}
                    }
                    res.push(tt);
                    cursor = next;
                }
                Ok((quote! { #ts #(#res)* }, cursor))
            })
        }

        fn insert_current_expr_value_into_list(list: &mut PunctuatedList, key: &mut AssignKey, value: &mut AssignValue, punct_span: Span) -> syn::Result<()> {
            match key.is_none() {
                true => Err(syn::Error::new(punct_span, "Comma wasn't expected, key was expected")),
                false => {
                    let attribute_assign = AttributeAssign {
                        key: key.take().unwrap(),
                        value: match value.take() {
                            None => return Err(syn::Error::new(punct_span, "Comma wasn't expected, value was expected")),
                            Some(Either::Left(expr)) => expr,
                            Some(Either::Right(ts)) => Expr::Verbatim(ts),
                        },
                    };
                    list.push(attribute_assign);
                    Ok(())
                }
            }
        }

        let mut list: PunctuatedList = Punctuated::new();
        let mut current_expr_assign_key: AssignKey = None;
        let mut current_expr_value: AssignValue = None;
        while !input.is_empty() {
            match input.peek(Token![,]) {
                true => {
                    let punct: Token![,] = input.parse()?;
                    match list.trailing_punct() {
                        false => insert_current_expr_value_into_list(&mut list, &mut current_expr_assign_key, &mut current_expr_value, punct.span)?,
                        true => list.push_punct(punct),
                    }
                }
                false => match current_expr_assign_key.is_some() {
                    true => {
                        match current_expr_value.take() {
                            None => match input.parse() {
                                Ok(expr) => current_expr_value.replace(Either::Left(expr)),
                                Err(_) => current_expr_value.replace(Either::Right(insert_tt_into_ts(TokenStream::new(), input)?)),
                            },
                            Some(Either::Left(expr)) => current_expr_value.replace(Either::Right(insert_tt_into_ts(expr, input)?)),
                            Some(Either::Right(_)) => unreachable!()
                        };
                    }
                    false => {
                        current_expr_assign_key.replace(input.parse()?);
                        let _: Token![=] = input.parse()?;
                    }
                }
            }
        }
        if let Some(_) = current_expr_value {
            insert_current_expr_value_into_list(&mut list, &mut current_expr_assign_key, &mut current_expr_value, Span::call_site())?;
        }
        let mut expressions = HashMap::new();
        for expr_assign in list {
            let left = expr_assign.key;
            let left_span = left.span();
            if let Some(_) = expressions.insert(left.to_string(), expr_assign.value) {
                return Err(syn::Error::new(left_span, "This key already used"));
            }
        }
        Ok(Self {
            expressions,
            span: input.span(),
        })
    }
}

impl Parse for GhostValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes: Attributes = input.parse()?;
        Ok(Self {
            value: attributes.remove_attribute(&"value".into())
                .map(|expr| expr.into_token_stream())
                .ok_or_else(|| syn::Error::new(input.span(), "Value must be provided"))?,
            ty: attributes.remove_ts_attribute(&"ty".into())?,
            order: match attributes.remove_attribute(&"order".into()).ok_or_else(|| syn::Error::new(input.span(), "Order must be provided"))? {
                Expr::Lit(lit) => match lit.lit {
                    Lit::Int(int) => GhostValueOrder::Order(int.base10_parse().unwrap(), int.span()),
                    _ => return Err(syn::Error::new(lit.span(), "Possible values are begin, end and order number")),
                },
                Expr::Path(path) => match path.path.is_ident("begin") {
                    true => GhostValueOrder::Begin,
                    false => match path.path.is_ident("end") {
                        true => GhostValueOrder::End,
                        false => return Err(syn::Error::new(path.span(), "Possible values are begin, end and order number")),
                    }
                },
                it => return Err(syn::Error::new(it.span(), "Possible values are begin, end and order number")),
            },
            variant: attributes.remove_ts_attribute(&"variant".into())?,
        })
    }
}

impl Parse for ObjectAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes: Attributes = input.parse()?;
        Ok(Self {
            key_variant: attributes.remove_ts_attribute(&"variant".into())?,
            key_ty: attributes.remove_ts_attribute(&"ty".into())?,
            key_increment: attributes.remove_ts_attribute(&"increment".into())?,
            key_reverse: attributes.remove_boolean_value(&"key_reverse".into(), false)?,
            packet_id: attributes.remove_ts_attribute(&"id".into())?,
            packet_bound: attributes.remove_ts_attribute(&"bound".into())?,
            packet_state: attributes.remove_ts_attribute(&"state".into())?,
            ghost_values: attributes.remove_ghost_values(&"ghost".into())?,
        })
    }
}

impl Parse for VariantAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes: Attributes = input.parse()?;
        Ok(Self {
            key_value: attributes.remove_ts_attribute(&"value".into())?,
            ghost_values: attributes.remove_ghost_values(&"ghost".into())?,
        })
    }
}

impl Parse for FieldAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes: Attributes = input.parse()?;
        Ok(Self {
            order: attributes.remove_str_parse_attribute(&"order".into())?,
            variant: attributes.remove_ts_attribute(&"variant".into())?,
        })
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn expr_path_into_string(path: &ExprPath) -> String {
    path.path.segments.iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<String>>()
        .join("::")
}

pub fn parse_attributes<A: Parse + Default>(attrs: &Vec<syn::Attribute>, attr_name: &str) -> syn::Result<A> {
    attrs.iter()
        .find(|attr| attr.path.is_ident(attr_name))
        .map(|attr| attr.parse_args())
        .unwrap_or_else(|| Ok(A::default()))
}

pub fn create_prepared_fields(fields: Fields, ghost_values: impl Iterator<Item=GhostValue>) -> syn::Result<Vec<(TokenStream, Option<TokenStream>, Option<TokenStream>, Option<TokenStream>)>> {
    let mut counter = 0;
    let mut begin = Vec::new();
    let mut end = Vec::new();
    let mut ordered_fields = Vec::new();
    let mut specific_ordered_fields = HashMap::new();
    for mut field in fields {
        if None == field.ident {
            field.ident.replace(Ident::new(format!("__{}", counter).as_str(), Span::call_site()));
            counter += 1;
        }
        let field_attributes: FieldAttributes = parse_attributes(&field.attrs, "bp")?;
        let to_insert = (field.ident.unwrap().into_token_stream(), None, Some(field.ty.into_token_stream()), field_attributes.variant);
        match field_attributes.order {
            Some((order, span)) => if let Some(_) = specific_ordered_fields.insert(order, to_insert) {
                return Err(syn::Error::new(span, "Repeated order value"));
            },
            None => ordered_fields.push(to_insert),
        }
    }
    for ghost_value in ghost_values {
        let to_insert = (quote! { _ }, Some(ghost_value.value), ghost_value.ty, ghost_value.variant);
        match ghost_value.order {
            GhostValueOrder::Begin => begin.push(to_insert),
            GhostValueOrder::End => end.push(to_insert),
            GhostValueOrder::Order(order, span) => if let Some(_) = specific_ordered_fields.insert(order, to_insert) {
                return Err(syn::Error::new(span, "Repeated order value"));
            }
        }
    }
    let mut specific_ordered_fields: Vec<_> = specific_ordered_fields.into_iter().collect();
    specific_ordered_fields.sort_by(|(first, _), (second, _)| first.cmp(second));
    for (order, obj) in specific_ordered_fields {
        ordered_fields.insert(order as usize, obj);
    }
    for begin in begin.into_iter().rev() {
        ordered_fields.insert(0, begin);
    }
    for end in end.into_iter() {
        ordered_fields.push(end)
    }
    Ok(ordered_fields)
}

pub fn create_prepared_variants(variants: impl Iterator<Item=Variant>, object_attributes: &ObjectAttributes) -> syn::Result<Vec<(Variant, TokenStream, VariantAttributes)>> {
    let mut result = Vec::new();
    let mut previous_value = quote! { 0 };
    let key_ty = object_attributes.key_ty.as_ref().unwrap();
    let increment = object_attributes.key_increment.clone().unwrap_or_else(|| quote! { + (1 as #key_ty) });
    for variant in variants {
        let variant_attributes: VariantAttributes = parse_attributes(&variant.attrs, "bp")?;
        let value = match variant_attributes.key_value {
            Some(ref value) => value.clone(),
            None => quote! { (#previous_value) as #key_ty  },
        };
        previous_value = quote! { #value #increment };
        result.push((variant, value, variant_attributes));
    }
    Ok(result)
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