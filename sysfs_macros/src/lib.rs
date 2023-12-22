#![allow(dead_code)]
#![allow(clippy::unit_arg)]

mod patterns;

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Brace;
use syn::{
    braced, parenthesized, parse_macro_input, Attribute, Error, Expr, ExprClosure, ExprLit, FnArg,
    Ident, Lit, LitStr, Meta, MetaNameValue, Pat, PatIdent, PatType, Token, Type, Visibility,
};

use self::patterns::Items;

mod kw {
    syn::custom_keyword!(sysfs_attr);
    syn::custom_keyword!(read);
    syn::custom_keyword!(write);
}

#[derive(Clone)]
struct ItemSysfsAttr {
    span: Span,
    meta_attrs: Vec<Attribute>,
    fn_vis: Visibility,
    attr_name: Ident,
    attr_path_args: Punctuated<FnArg, Token![,]>,
    sysfs_dir: Option<LitStr>,
    getter: Option<GetterSignature>,
    setter: Option<SetterSignature>,
}

#[derive(Clone)]
struct GetterSignature {
    span: Span,
    parse_fn: Expr,
    into_type: Box<Type>,
}

struct GetterFunction {
    span: Span,
    meta_attrs: Vec<Attribute>,
    fn_vis: Visibility,
    attr_name: Ident,
    attr_path_args: Punctuated<FnArg, Token![,]>,
    sysfs_dir: LitStr,
    parse_fn: Expr,
    into_type: Box<Type>,
}

#[derive(Clone)]
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
    attr_path_args: Punctuated<FnArg, Token![,]>,
    sysfs_dir: LitStr,
    format_fn: ExprClosure,
    from_ident: Ident,
    from_type: Box<Type>,
}

impl Parse for ItemSysfsAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut sysfs_attr = Self {
            span: input.span(),
            meta_attrs: Attribute::parse_outer(input)?,
            fn_vis: Visibility::parse(input)?,
            attr_name: kw::sysfs_attr::parse(input).and_then(|_| Ident::parse(input))?,
            attr_path_args: {
                let args;
                parenthesized!(args in input);
                args.parse_terminated(FnArg::parse, Token![,])?
            },
            sysfs_dir: <Token![in]>::parse(input)
                .ok()
                .map(|_| match Lit::parse(input) {
                    Ok(Lit::Str(sysfs_path)) => Ok(sysfs_path),
                    _ => Err(Error::new(input.span(), "expected a string literal")),
                })
                .transpose()?,
            getter: None,
            setter: None,
        };

        let braced;
        braced!(braced in input);

        if braced.peek(kw::read) {
            kw::read::parse(&braced)?;
            <Token![:]>::parse(&braced)?;
            sysfs_attr.getter = Some(braced.parse()?);
            <Token![,]>::parse(&braced)?;
        }

        if braced.peek(kw::write) {
            kw::write::parse(&braced)?;
            <Token![:]>::parse(&braced)?;
            sysfs_attr.setter = Some(braced.parse()?);
            <Token![,]>::parse(&braced)?;
        }

        Ok(sysfs_attr)
    }
}

impl Parse for GetterSignature {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;

        Ok(Self {
            span: expr.span(),
            parse_fn: expr,
            into_type: {
                <Token![=>]>::parse(input)?;
                Box::new(Type::parse(input)?)
            },
        })
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

impl TryFrom<ItemSysfsAttr> for GetterFunction {
    type Error = Error;

    fn try_from(sysfs_attr: ItemSysfsAttr) -> Result<Self, Self::Error> {
        let getter = sysfs_attr.getter.ok_or(Error::new(
            sysfs_attr.span,
            "provided `AttributeItem` has no `getter` closure",
        ))?;
        let sysfs_dir = sysfs_attr.sysfs_dir.ok_or(Error::new(
            sysfs_attr.span,
            "provided `AttributeItem` has no `sysfs_dir` literal",
        ))?;
        Ok(Self {
            span: sysfs_attr.span,
            meta_attrs: sysfs_attr.meta_attrs,
            fn_vis: sysfs_attr.fn_vis,
            attr_name: sysfs_attr.attr_name,
            attr_path_args: sysfs_attr.attr_path_args,
            sysfs_dir,
            parse_fn: getter.parse_fn,
            into_type: getter.into_type,
        })
    }
}

impl TryFrom<ItemSysfsAttr> for SetterFunction {
    type Error = Error;

    fn try_from(sysfs_attr: ItemSysfsAttr) -> Result<Self, Self::Error> {
        let setter = sysfs_attr.setter.ok_or(Error::new(
            sysfs_attr.span,
            "provided `AttributeItem` has no `setter` closure",
        ))?;
        let sysfs_dir = sysfs_attr.sysfs_dir.ok_or(Error::new(
            sysfs_attr.span,
            "provided `AttributeItem` has no `sysfs_dir` literal",
        ))?;
        Ok(Self {
            span: sysfs_attr.span,
            meta_attrs: sysfs_attr.meta_attrs,
            fn_vis: sysfs_attr.fn_vis,
            attr_name: sysfs_attr.attr_name,
            attr_path_args: sysfs_attr.attr_path_args,
            sysfs_dir,
            format_fn: setter.format_fn,
            from_ident: setter.from_ident,
            from_type: setter.from_type,
        })
    }
}

impl ToTokens for GetterFunction {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
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
            #fn_vis fn #attr_name(#(#attr_path_args,)*) -> sysfs::Result<#into_type> {
                let file_path = format!("{}/{}", format_args!(#sysfs_dir), stringify!(#attr_name));
                unsafe {
                    sysfs::sysfs_read::<#into_type>(&file_path, #parse_fn)
                }
            }
        ))
    }
}

impl ToTokens for SetterFunction {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
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
            #[allow(clippy::redundant_closure_call)]
            #fn_vis fn #setter_ident(#(#attr_path_args,)* #from_ident: #from_type) -> sysfs::Result<()> {
                let file_path = format!("{}/{}", format_args!(#sysfs_dir), stringify!(#attr_name));
                sysfs::sysfs_write(&file_path, (#format_fn)(#from_ident))
            }
        ));
    }
}

#[proc_macro]
pub fn impl_sysfs_attrs(tokens: TokenStream1) -> TokenStream1 {
    match parse_macro_input!(tokens as Items<ItemSysfsAttr>) {
        Items::Braced { brace_token, .. } => {
            Error::new(brace_token.span.span(), "unexpected brace")
                .to_compile_error()
                .into()
        }
        Items::TopLevel { attrs, items } => {
            let mut tokens = TokenStream2::new();
            for attr in attrs {
                attr.to_tokens(&mut tokens)
            }
            for sysfs_attr in items {
                if let Ok(getter) = GetterFunction::try_from(sysfs_attr.clone()) {
                    tokens.extend(quote_spanned!(getter.span => #getter));
                }
                if let Ok(setter) = SetterFunction::try_from(sysfs_attr.clone()) {
                    tokens.extend(quote_spanned!(setter.span => #setter));
                }
            }
            tokens.into()
        }
    }
}

#[derive(Default)]
struct SysfsModArgs {
    sysfs_dir: Option<LitStr>,
}

impl Parse for SysfsModArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut sysfs_dir = None;

        let meta = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        meta.into_iter().try_for_each(|nested| match nested {
            Meta::NameValue(MetaNameValue { path, value, .. }) if path.is_ident("sysfs_dir") => {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(literal),
                    ..
                }) = value
                {
                    Ok(sysfs_dir = Some(literal))
                } else {
                    Err(Error::new(value.span(), "expected a string literal"))
                }
            }
            _ => Err(Error::new(nested.span(), "unknown meta")),
        })?;

        Ok(Self { sysfs_dir })
    }
}

struct ItemSysfsMod {
    span: Span,
    attrs: Vec<Attribute>,
    vis: Visibility,
    unsafety: Option<Token![unsafe]>,
    mod_token: Token![mod],
    ident: Ident,
    brace: Brace,
    items: Vec<ItemSysfsAttr>,
}

impl Parse for ItemSysfsMod {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = Attribute::parse_outer(input)?;
        let vis = input.parse()?;
        let unsafety = input.parse()?;
        let mod_token = input.parse()?;
        let ident = input.parse()?;
        let (brace, items) = {
            let braced;
            let brace = braced!(braced in input);
            attrs.append(&mut Attribute::parse_inner(&braced)?);
            let mut items = Vec::new();
            while !braced.is_empty() {
                items.push(braced.parse()?)
            }
            (brace, items)
        };
        Ok(ItemSysfsMod {
            span: input.span(),
            attrs,
            vis,
            unsafety,
            mod_token,
            brace,
            ident,
            items,
        })
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
            let mut tokens = TokenStream2::new();
            let parsed: $parse_ty = parse_quote!($($input)*);
            parsed.to_tokens(&mut tokens);
            println!("parsed {}: {}", stringify!($parse_ty), tokens)
        }};
    }

    #[test]
    fn empty_sysfs_attr_parses() {
        test_parse!({
            pub sysfs_attr some_useless_attr(item: usize) in "/fake/sysfs/path/item{item}" {}
        } => ItemSysfsAttr);
    }

    #[test]
    fn readonly_sysfs_attr_parses() {
        test_parse!({
            pub sysfs_attr some_readonly_attr(item: usize) in "/fake/sysfs/path/item{item}" {
                read: |text| text.parse().unwrap() => f32,
            }
        } => ItemSysfsAttr);
    }

    #[test]
    fn readwrite_sysfs_attr_parses() {
        test_parse!({
            pub sysfs_attr some_readwrite_attr(item: usize) in "/fake/sysfs/path/boolean{item}" {
                read: |text| text.parse().unwrap() => bool,
                write: |value: bool| format_args!("{}", value as u8),
            }
        } => ItemSysfsAttr);
    }

    #[test]
    fn getter_closure_parses() {
        test_parse!({
            |text| text.parse().unwrap() => isize
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
        let mut tokens = TokenStream2::new();
        let sysfs_attr: ItemSysfsAttr = parse_quote! {
            pub sysfs_attr some_readonly_attr(item: usize) in "/fake/sysfs/path/item{item}" {
                read: |text| text.parse().unwrap() => f32,
            }
        };
        GetterFunction::try_from(sysfs_attr)
            .unwrap_or_else(|error| panic!("{}", error))
            .to_tokens(&mut tokens);
        eprintln!("parsed {}: {}", stringify!(GetterFunction), tokens)
    }

    #[test]
    fn readwrite_sysfs_attr_roundtrips() {
        let mut tokens = TokenStream2::new();
        let sysfs_attr: ItemSysfsAttr = parse_quote! {
            pub sysfs_attr some_write_attr(item: usize) in "/fake/sysfs/path/item{item}" {
                read: |text| text.parse().unwrap() => f32,
                write: |value: f32| fornat!("{value}"),
            }
        };
        GetterFunction::try_from(sysfs_attr.clone())
            .unwrap_or_else(|error| panic!("{}", error))
            .to_tokens(&mut tokens);
        SetterFunction::try_from(sysfs_attr.clone())
            .unwrap_or_else(|error| panic!("{}", error))
            .to_tokens(&mut tokens);
        eprintln!("parsed {}: {}", stringify!(GetterFunction), tokens)
    }

    #[test]
    fn attr_mod_parses() {
        test_parse!({
            pub mod cpufreq {
                /// This example is from the linux kernel.
                pub sysfs_attr scaling_max_freq(cpu: usize) in "{SYSFS_DIR}/policy{cpu}" {
                    read: |text| text.parse().unwrap() => usize,
                    write: |freq: usize| format!("{freq}"),
                }
            }
        } => ItemSysfsMod);
    }

    #[test]
    fn attr_mod_with_args_parses() {
        let _: ItemSysfsMod = parse_quote! {
            pub mod cpufreq {
                /// This example is from the linux kernel.
                pub sysfs_attr scaling_max_freq(cpu: usize) in "" {
                    read: |text| text.parse().unwrap() => usize,
                    write: |freq: usize| format!("{freq}"),
                }
            }
        };
    }

    #[test]
    fn attr_item_mod_args_parses() {
        let args: SysfsModArgs = parse_quote!(sysfs_dir = "/some/sysfs/dir");
        let sysfs_dir = args.sysfs_dir.unwrap().value();
        assert_eq!(sysfs_dir, "/some/sysfs/dir");
    }
}
