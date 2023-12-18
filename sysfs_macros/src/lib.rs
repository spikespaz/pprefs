#![allow(dead_code)]

use std::fmt;

use proc_macro2::Span;
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Colon, Comma, FatArrow, In};
use syn::{
    braced, parenthesized, Attribute, Error, Expr, ExprClosure, FnArg, Ident, Lit, LitStr,
    ReturnType, Type, Visibility,
};

mod kw {
    syn::custom_keyword!(sysfs_attr);
    syn::custom_keyword!(read);
    syn::custom_keyword!(write);
}

struct AttributeItem {
    span: Span,
    meta_attrs: Vec<Attribute>,
    fn_vis: Visibility,
    attr_name: Ident,
    attr_path_args: Punctuated<FnArg, Comma>,
    sysfs_dir: LitStr,
    getter: Option<GetterSignature>,
}

impl Parse for AttributeItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            span: input.span(),
            meta_attrs: Attribute::parse_outer(input)?,
            fn_vis: Visibility::parse(input)?,
            attr_name: kw::sysfs_attr::parse(input).and_then(|_| Ident::parse(input))?,
            attr_path_args: {
                let args;
                parenthesized!(args in input);
                args.parse_terminated(FnArg::parse, Comma)?
            },
            sysfs_dir: In::parse(input).and_then(|_| match Lit::parse(input) {
                Ok(Lit::Str(sysfs_path)) => Ok(sysfs_path),
                _ => Err(Error::new(input.span(), "expected a string literal")),
            })?,
            getter: {
                let braced;
                braced!(braced in input);
                if braced.peek(kw::read) {
                    kw::read::parse(&braced)?;
                    Colon::parse(&braced)?;
                    let getter = braced.parse()?;
                    Comma::parse(&braced)?;
                    Some(getter)
                } else {
                    None
                }
            },
        })
    }
}

impl fmt::Debug for AttributeItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            meta_attrs,
            fn_vis,
            attr_name,
            attr_path_args,
            sysfs_dir,
            getter,
            ..
        } = self;
        let attr_path_args = attr_path_args.into_iter();
        write!(
            f,
            "{}",
            quote! {
                #(#meta_attrs)*
                #fn_vis sysfs_attr #attr_name(#(#attr_path_args)*) in #sysfs_dir {

                }
            }
        )
    }
}

struct GetterSignature {
    span: Span,
    parse_fn: ExprClosure,
    into_type: Box<Type>,
}

impl Parse for GetterSignature {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;
        match &expr {
            Expr::Closure(parse_fn) => {
                let output = match &parse_fn.output {
                    ReturnType::Type(_, ty) => ty.clone(),
                    ReturnType::Default => {
                        FatArrow::parse(input)?;
                        Box::new(Type::parse(input)?)
                    }
                };
                Ok(Self {
                    span: parse_fn.span(),
                    parse_fn: parse_fn.clone(),
                    into_type: output,
                })
            }
            _ => Err(Error::new(expr.span(), "expected a function closure")),
        }
    }
}

impl ToTokens for GetterSignature {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self { span, parse_fn, .. } = self;
        tokens.extend(quote_spanned!(*span =>
            #parse_fn
        ))
    }
}

impl fmt::Debug for GetterSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            parse_fn,
            into_type,
            ..
        } = self;
        write!(
            f,
            "{}",
            quote!(GetterFunction {
                parse_fn: #parse_fn,
                into_type: #into_type,
            })
        )
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[rustfmt::skip]
    macro_rules! test_parse {
        ({ $($input:tt)* } => $parse_ty:ty) => {{
            let result: syn::Result<$parse_ty> = syn::parse_str(&(quote::quote!{
                $($input)*
            }).to_string());
            match result {
                Ok(parsed) => dbg!(parsed),
                Err(e) => panic!("{}", e.to_string()),
            }
        }};
    }

    #[rustfmt::skip]
    macro_rules! test_roundtrip {
        ({ $($input:tt)* } => $parse_ty:ty) => {{
            let mut tokens = proc_macro2::TokenStream::new();
            let getter: $parse_ty = parse_quote!($($input)*);
            getter.to_tokens(&mut tokens);
            eprintln!("parsed {}: {}", stringify!($parse_ty), tokens)
        }};
    }

    #[test]
    fn empty_sysfs_attr_parses() {
        test_parse!({
            pub sysfs_attr some_useless_attr(item: usize) in "/fake/sysfs/path/item{item}" {}
        } => AttributeItem);
    }

    #[test]
    fn readonly_sysfs_attr_parses() {
        test_parse!({
            pub sysfs_attr some_readonly_attr(item: usize) in "/fake/sysfs/path/item{item}" {
                read: |text| text.parse().unwrap() => f32,
            }
        } => AttributeItem);
        test_parse!({
            pub sysfs_attr some_readonly_attr(item: usize) in "/fake/sysfs/path/item{item}" {
                read: |text| -> f32 { text.parse().unwrap() },
            }
        } => AttributeItem);
    }

    #[test]
    fn getter_closure_parses() {
        // With custom fat arrow return type syntax.
        test_parse!({
            |text| text.parse().unwrap() => isize
        } => GetterSignature);
        // With native Rust return type syntax.
        test_parse!({
            |text| -> isize { text.parse().unwrap() }
        } => GetterSignature);
    }

    #[test]
    fn getter_roundtrip() {
        // With custom fat arrow return type syntax.
        test_roundtrip!({
            |text| text.parse().unwrap() => isize
        } => GetterSignature);
        // With native Rust return type syntax.
        test_roundtrip!({
            |text| -> isize { text.parse().unwrap() }
        } => GetterSignature);
    }
}
