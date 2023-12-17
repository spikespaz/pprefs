#![allow(dead_code)]

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Colon, Comma, In};
use syn::{
    braced, parenthesized, parse_macro_input, Attribute, Block, Error, Expr, ExprClosure, Field,
    Fields, FieldsNamed, FnArg, Ident, Lit, LitStr, Pat, PatIdent, PatType, ReturnType, Type,
    Visibility,
};

mod kw {
    syn::custom_keyword!(sysfs_attr);
    syn::custom_keyword!(read);
    syn::custom_keyword!(write);
}

struct SysfsAttribute {
    meta_attrs: Vec<Attribute>,
    fn_vis: Visibility,
    attr_name: Ident,
    attr_path_args: Punctuated<FnArg, Comma>,
    sysfs_dir: LitStr,
    getter: Option<GetterFunction>,
}

impl Parse for SysfsAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let meta_attrs = Attribute::parse_outer(input)?;
        let fn_vis = Visibility::parse(input)?;
        kw::sysfs_attr::parse(input)?;
        let attr_name = Ident::parse(input)?;
        let args;
        parenthesized!(args in input);
        let attr_path_args = args.parse_terminated(FnArg::parse, Comma)?;
        In::parse(input)?;
        let sysfs_dir = Lit::parse(input).and_then(|lit| match lit {
            Lit::Str(sysfs_path) => Ok(sysfs_path),
            _ => Err(Error::new(lit.span(), "expected a string literal")),
        })?;

        let braced;
        braced!(braced in input);

        let getter = if braced.peek(kw::read) {
            kw::read::parse(&braced)?;
            Colon::parse(&braced)?;
            Some(braced.parse()?)
        } else {
            None
        };

        if getter.is_some() {
            Comma::parse(&braced)?;
        }

        Ok(Self {
            meta_attrs,
            fn_vis,
            attr_name,
            attr_path_args,
            sysfs_dir,
            getter,
        })
    }
}

struct GetterFunction {
    parse_fn: ExprClosure,
    into_type: ReturnType,
}

impl Parse for GetterFunction {
    #[rustfmt::skip]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Expr::parse(input).and_then(|expr| match &expr {
            Expr::Closure(closure) => Ok(Self {
                parse_fn: closure.clone(),
                into_type: closure.output.clone(),
            }),
            _ => Err(Error::new(expr.span(), "expected a function closure")),
        })
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    #[rustfmt::skip]
    macro_rules! test_parse {
        ($parse_ty:ty, $input:expr) => {{
            let result: syn::Result<$parse_ty> = syn::parse_str(&($input).to_string());
            if let Err(e) = result {
                panic!("{}", e.to_string());
            }
        }};
    }

    #[test]
    fn empty_sysfs_attr_parses() {
        test_parse!(
            SysfsAttribute,
            quote! {
                pub sysfs_attr some_useless_attr(item: usize) in "/fake/sysfs/path/item{item}" {}
            }
        );
    }

    #[test]
    fn getter_closure_parses() {
        test_parse!(
            GetterFunction,
            quote! {
                |text| text.parse().unwrap()
            }
        )
    }
}
