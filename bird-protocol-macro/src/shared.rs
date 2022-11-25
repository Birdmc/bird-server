use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::str::FromStr;
use either::Either;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{Expr, ExprAssign, ExprPath, ExprType, Field, Fields, GenericParam, Generics, Lifetime, LifetimeDef, Lit, Token, Type};
use syn::parse::{Parse, ParseStream};
use syn::parse::discouraged::Speculative;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Token;

pub struct ObjectAttributes {}

pub struct VariantAttributes {}

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

    pub fn remove_type_attribute(&mut self, name: &String) -> syn::Result<Option<TokenStream>> {
        match self.remove_attribute(name) {
            // Unchecked because i don't know how to made it checked using Expr enum
            Some(expr) => Ok(Some(expr.to_token_stream())),
            None => Ok(None),
        }
    }
}


impl Parse for Attributes {
    fn parse(mut input: ParseStream) -> syn::Result<Self> {
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
                            ',' => if depth <= 0 {
                                break;
                            }
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

impl Parse for FieldAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes: Attributes = input.parse()?;
        Ok(Self {
            order: attributes.remove_str_parse_attribute(&"order".into())?,
            variant: attributes.remove_type_attribute(&"variant".into())?,
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
                return Err(syn::Error::new(span, "Repeated order value"));
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