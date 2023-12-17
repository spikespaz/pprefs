#![allow(dead_code)]

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::{Comma, In};
use syn::{braced, parenthesized, Attribute, Error, FnArg, Ident, Lit, LitStr, Visibility};

mod kw {
    syn::custom_keyword!(sysfs_attr);
    syn::custom_keyword!(read);
    syn::custom_keyword!(write);
}

struct SysfsAttribute {
    span: Span,
    meta_attrs: Vec<Attribute>,
    fn_vis: Visibility,
    attr_name: Ident,
    attr_path_args: Punctuated<FnArg, Comma>,
    sysfs_dir: LitStr,
    getter: Option<()>,
    setter: Option<()>,
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

        Ok(Self {
            span: input.span(),
            meta_attrs,
            fn_vis,
            attr_name,
            attr_path_args,
            sysfs_dir,
            getter: None,
            setter: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    #[rustfmt::skip]
    macro_rules! test_parse {
        ($input:expr) => {{
            let result: syn::Result<SysfsAttribute> = syn::parse_str(&($input).to_string());
            if let Err(e) = result {
                panic!("{}", e.to_string());
            }
        }};
    }

    #[test]
    fn empty_sysfs_attr_compiles() {
        test_parse!(quote! {
            pub sysfs_attr some_useless_attr(item: usize) in "/fake/sysfs/path/item{item}" {}
        });
    }
}
