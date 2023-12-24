#![allow(dead_code)]
#![allow(clippy::unit_arg)]

mod patterns;

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, Attribute, Block, Error, Expr, ExprClosure, ExprLit, ExprRange, FnArg,
    Ident, ItemFn, Lit, LitStr, Local, LocalInit, Meta, MetaNameValue, Pat, PatIdent, PatType,
    RangeLimits, Signature, Stmt, Token, Type, Visibility,
};

#[derive(Default)]
struct ItemSysfsAttrArgs {
    sysfs_dir: Option<LitStr>,
}

impl Parse for ItemSysfsAttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Self::default())
        } else if input.peek(Token![in]) {
            let _in_token = <Token![in]>::parse(input)?;
            match Lit::parse(input)? {
                Lit::Str(lit) => Ok(Self {
                    sysfs_dir: Some(lit),
                }),
                lit => Err(Error::new(lit.span(), "expected a literal string")),
            }
        } else {
            // match Punctuated::<Meta, Token![,]>::parse_terminated.parse(input)
            todo!("parse meta")
        }
    }
}

#[proc_macro_attribute]
pub fn sysfs(args: TokenStream1, item: TokenStream1) -> TokenStream1 {
    let args = parse_macro_input!(args as ItemSysfsAttrArgs);
    let item = parse_macro_input!(item as ItemFn);

    let ItemSysfsAttrFn {
        attrs,
        vis,
        sig,
        let_read,
        let_write,
        dots,
        block,
    } = match ItemSysfsAttrFn::try_from(item) {
        Ok(item) => item,
        Err(e) => return e.to_compile_error().into(),
    };

    let mut body = TokenStream2::new();

    if let Some((local, _)) = let_read {
        local.to_tokens(&mut body)
    }

    if let Some((local, _)) = let_write {
        local.to_tokens(&mut body)
    }

    quote! {
        #(#attrs)*
        #vis #sig {
            #body
        }
    }
    .into()
    // TokenStream1::new()
}

#[derive(Clone)]
struct ItemSysfsAttrFn {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
    let_read: Option<(Local, Box<Expr>)>,
    let_write: Option<(Local, Box<Expr>)>,
    dots: Token![..],
    block: Box<Block>,
}

impl TryFrom<ItemFn> for ItemSysfsAttrFn {
    type Error = Error;

    fn try_from(
        ItemFn {
            attrs,
            vis,
            sig,
            block,
        }: ItemFn,
    ) -> Result<Self, Self::Error> {
        let let_read = block
            .stmts
            .iter()
            .filter(|stmt| {
                matches!(stmt, Stmt::Local(Local {
                    pat: Pat::Ident(PatIdent { ident, .. }),
                    init: Some(LocalInit { expr, .. }),
                    ..
                }) if ident == "read" && matches!(expr.as_ref(), Expr::Closure(_)))
            })
            .last()
            .map(|stmt| match stmt {
                Stmt::Local(
                    local @ Local {
                        init: Some(LocalInit { expr, .. }),
                        ..
                    },
                ) => (local.clone(), expr.clone()),
                _ => unreachable!(),
            });
        let let_write = block
            .stmts
            .iter()
            .filter(|stmt| {
                matches!(stmt, Stmt::Local(Local {
                    pat: Pat::Ident(PatIdent { ident, .. }),
                    init: Some(LocalInit { expr, .. }),
                    ..
                }) if ident == "write" && matches!(expr.as_ref(), Expr::Closure(_)))
            })
            .last()
            .map(|stmt| match stmt {
                Stmt::Local(
                    local @ Local {
                        init: Some(LocalInit { expr, .. }),
                        ..
                    },
                ) => (local.clone(), expr.clone()),
                _ => unreachable!(),
            });
        let dots = match block.stmts.iter().last() {
            Some(Stmt::Expr(
                Expr::Range(ExprRange {
                    attrs,
                    start: None,
                    limits: RangeLimits::HalfOpen(dots),
                    end: None,
                }),
                None,
            )) if attrs.is_empty() => Ok(*dots),
            _ => Err(Error::new(
                block.span(),
                "expected `..` to be the return expression",
            )),
        }?;

        Ok(Self {
            attrs,
            vis,
            sig,
            let_read,
            let_write,
            dots,
            block,
        })
    }
}

mod kw {
    syn::custom_keyword!(sysfs_attr);
    syn::custom_keyword!(read);
    syn::custom_keyword!(write);
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

// #[proc_macro]
// pub fn impl_sysfs_attrs(tokens: TokenStream1) -> TokenStream1 {
//     match parse_macro_input!(tokens as Items<ItemSysfsAttr>) {
//         Items::Braced { brace_token, .. } => {
//             Error::new(brace_token.span.span(), "unexpected brace")
//                 .to_compile_error()
//                 .into()
//         }
//         Items::TopLevel { attrs, items } => {
//             let mut tokens = TokenStream2::new();
//             for attr in attrs {
//                 attr.to_tokens(&mut tokens)
//             }
//             for sysfs_attr in items {
//                 if let Ok(getter) = GetterFunction::try_from(sysfs_attr.clone()) {
//                     tokens.extend(quote_spanned!(getter.span => #getter));
//                 }
//                 if let Ok(setter) = SetterFunction::try_from(sysfs_attr.clone()) {
//                     tokens.extend(quote_spanned!(setter.span => #setter));
//                 }
//             }
//             tokens.into()
//         }
//     }
// }

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

// struct ItemSysfsMod {
//     span: Span,
//     attrs: Vec<Attribute>,
//     vis: Visibility,
//     unsafety: Option<Token![unsafe]>,
//     mod_token: Token![mod],
//     ident: Ident,
//     brace: Brace,
//     items: Vec<ItemSysfsAttr>,
// }

// impl Parse for ItemSysfsMod {
//     fn parse(input: ParseStream) -> syn::Result<Self> {
//         let mut attrs = Attribute::parse_outer(input)?;
//         let vis = input.parse()?;
//         let unsafety = input.parse()?;
//         let mod_token = input.parse()?;
//         let ident = input.parse()?;
//         let (brace, items) = {
//             let braced;
//             let brace = braced!(braced in input);
//             attrs.append(&mut Attribute::parse_inner(&braced)?);
//             let mut items = Vec::new();
//             while !braced.is_empty() {
//                 items.push(braced.parse()?)
//             }
//             (brace, items)
//         };
//         Ok(ItemSysfsMod {
//             span: input.span(),
//             attrs,
//             vis,
//             unsafety,
//             mod_token,
//             brace,
//             ident,
//             items,
//         })
//     }
// }

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
}
