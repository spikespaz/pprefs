#![allow(dead_code)]

use proc_macro2::Span;
use quote::{format_ident, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Colon, Comma, FatArrow, In};
use syn::{
    braced, parenthesized, Attribute, Error, Expr, ExprClosure, FnArg, Ident, Lit, LitStr, Pat,
    PatIdent, PatType, ReturnType, Type, Visibility,
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
    setter: Option<SetterSignature>,
}

struct GetterSignature {
    span: Span,
    parse_fn: ExprClosure,
    into_type: Box<Type>,
}

struct GetterFunction {
    span: Span,
    meta_attrs: Vec<Attribute>,
    fn_vis: Visibility,
    attr_name: Ident,
    attr_path_args: Punctuated<FnArg, Comma>,
    sysfs_dir: LitStr,
    parse_fn: ExprClosure,
    into_type: Box<Type>,
}

struct SetterSignature {
    span: Span,
    format_fn: ExprClosure,
    from_ident: Ident,
    from_type: Box<Type>,
}

struct SetterFunction {
    span: Span,
    meta_attrs: Vec<Attribute>,
    fn_vis: Visibility,
    attr_name: Ident,
    attr_path_args: Punctuated<FnArg, Comma>,
    sysfs_dir: LitStr,
    format_fn: ExprClosure,
    from_ident: Ident,
    from_type: Box<Type>,
}

impl Parse for AttributeItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut sysfs_attr = Self {
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
            getter: None,
            setter: None,
        };

        let braced;
        braced!(braced in input);

        if braced.peek(kw::read) {
            kw::read::parse(&braced)?;
            Colon::parse(&braced)?;
            sysfs_attr.getter = Some(braced.parse()?);
            Comma::parse(&braced)?;
        }

        if braced.peek(kw::write) {
            kw::write::parse(&braced)?;
            Colon::parse(&braced)?;
            sysfs_attr.setter = Some(braced.parse()?);
            Comma::parse(&braced)?;
        }

        Ok(sysfs_attr)
    }
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

impl Parse for SetterSignature {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;
        match &expr {
            Expr::Closure(parse_fn) => {
                let first_input = parse_fn.inputs.first().ok_or_else(|| {
                    Error::new(parse_fn.span(), "expected at least one argument in closure")
                })?;

                let (from_ident, from_type) =
                    if let Pat::Type(PatType { pat, ty, .. }) = first_input {
                        if let Pat::Ident(PatIdent { ident, .. }) = pat.as_ref() {
                            Ok((ident, ty))
                        } else {
                            Err(Error::new(pat.span(), "expected a single typed identifier"))
                        }
                    } else {
                        Err(Error::new(
                            first_input.span(),
                            "expected a single typed identifier",
                        ))
                    }?;

                Ok(Self {
                    span: parse_fn.span(),
                    format_fn: parse_fn.clone(),
                    from_ident: from_ident.clone(),
                    from_type: from_type.clone(),
                })
            }
            _ => Err(Error::new(expr.span(), "expected a function closure")),
        }
    }
}

impl TryFrom<AttributeItem> for GetterFunction {
    type Error = &'static str;

    fn try_from(sysfs_attr: AttributeItem) -> Result<Self, Self::Error> {
        sysfs_attr
            .getter
            .ok_or("provided SysfsAttribute has no getter")
            .map(|getter| {
                Ok(Self {
                    span: sysfs_attr.span,
                    meta_attrs: sysfs_attr.meta_attrs,
                    fn_vis: sysfs_attr.fn_vis,
                    attr_name: sysfs_attr.attr_name,
                    attr_path_args: sysfs_attr.attr_path_args,
                    sysfs_dir: sysfs_attr.sysfs_dir,
                    parse_fn: getter.parse_fn,
                    into_type: getter.into_type,
                })
            })?
    }
}

impl TryFrom<AttributeItem> for SetterFunction {
    type Error = &'static str;

    fn try_from(sysfs_attr: AttributeItem) -> Result<Self, Self::Error> {
        sysfs_attr
            .setter
            .ok_or("provided SysfsAttribute has no setter")
            .map(|setter| {
                Ok(Self {
                    span: sysfs_attr.span,
                    meta_attrs: sysfs_attr.meta_attrs,
                    fn_vis: sysfs_attr.fn_vis,
                    attr_name: sysfs_attr.attr_name,
                    attr_path_args: sysfs_attr.attr_path_args,
                    sysfs_dir: sysfs_attr.sysfs_dir,
                    format_fn: setter.format_fn,
                    from_ident: setter.from_ident,
                    from_type: setter.from_type,
                })
            })?
    }
}

impl ToTokens for GetterFunction {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            span,
            meta_attrs,
            fn_vis,
            attr_name,
            attr_path_args,
            sysfs_dir,
            parse_fn,
            into_type,
        } = self;
        let attr_path_args = attr_path_args.iter();
        tokens.extend(quote_spanned!(*span =>
            #(#meta_attrs)*
            #fn_vis #attr_name(#(#attr_path_args)*) -> sysfs::Result<#into_type> {
                let file_path = format!("{}/{}", format_args!(#sysfs_dir), stringify!(#attr_name));
                unsafe {
                    $crate::sysfs_read::<#into_type>(&file_path, #parse_fn)
                }
            }
        ))
    }
}

impl ToTokens for SetterFunction {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            span,
            meta_attrs,
            fn_vis,
            attr_name,
            attr_path_args,
            sysfs_dir,
            format_fn,
            from_ident,
            from_type,
        } = self;
        let attr_path_args = attr_path_args.iter();
        let setter_ident = format_ident!("set_{}", attr_name);
        tokens.extend(quote_spanned!(*span =>
            #(#meta_attrs)*
            #fn_vis #setter_ident(#(#attr_path_args)* #from_ident: #from_type) -> sysfs::Result<()> {
                let file_path = format!("{}/{}", format_args!(#sysfs_dir), stringify!(#attr_name));
                sysfs::sysfs_write(&file_path, #format_fn)
            }
        ));
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
                Ok(_) => (),
                Err(e) => panic!("{}", e.to_string()),
            }
        }};
    }

    #[rustfmt::skip]
    macro_rules! test_roundtrip {
        ({ $($input:tt)* } => $parse_ty:ty) => {{
            let mut tokens = proc_macro2::TokenStream::new();
            let parsed: $parse_ty = parse_quote!($($input)*);
            parsed.to_tokens(&mut tokens);
            println!("parsed {}: {}", stringify!($parse_ty), tokens)
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
    fn readwrite_sysfs_attr_parses() {
        test_parse!({
            pub sysfs_attr some_readonly_attr(item: usize) in "/fake/sysfs/path/boolean{item}" {
                read: |text| text.parse().unwrap() => bool,
                write: |value: bool| format_args!("{}", value as u8),
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
    fn setter_closure_parses() {
        test_parse!({
            |frequency: usize| format_args!("{}", frequency)
        } => SetterSignature);
        test_parse!({
            |number: i32| format_args!("{number}")
        } => SetterSignature);
        test_parse!({
            |flag: bool| format_args!("{}", bool as u8)
        } => SetterSignature);
    }

    #[test]
    fn readonly_sysfs_attr_roundtrips() {
        let mut tokens = proc_macro2::TokenStream::new();
        let sysfs_attr: AttributeItem = parse_quote! {
            pub sysfs_attr some_readonly_attr(item: usize) in "/fake/sysfs/path/item{item}" {
                read: |text| text.parse().unwrap() => f32,
            }
        };
        let getter =
            GetterFunction::try_from(sysfs_attr).unwrap_or_else(|error| panic!("{}", error));
        getter.to_tokens(&mut tokens);
        eprintln!("parsed {}: {}", stringify!(GetterFunction), tokens)
    }
}
