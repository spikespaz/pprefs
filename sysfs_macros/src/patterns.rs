use syn::parse::{Parse, ParseStream};
use syn::token::Brace;
use syn::{braced, Attribute};

pub(crate) enum Items<T> {
    Braced {
        attrs: Vec<Attribute>,
        brace_token: Brace,
        items: Vec<T>,
    },
    TopLevel {
        attrs: Vec<Attribute>,
        items: Vec<T>,
    },
}

impl<T: Parse> Items<T> {
    fn parse_inner(input: ParseStream) -> syn::Result<Self> {
        Ok(Self::TopLevel {
            attrs: Attribute::parse_inner(input)?,
            items: {
                let mut items = Vec::new();
                while !input.is_empty() {
                    items.push(input.parse()?)
                }
                items
            },
        })
    }
}

impl<T: Parse> Parse for Items<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let look = input.lookahead1();

        let mut outer_attrs = Attribute::parse_outer(input)?;

        if look.peek(Brace) {
            let braced;
            let brace_token = braced!(braced in input);
            match Self::parse_inner(&braced)? {
                Self::TopLevel { mut attrs, items } => Ok(Self::Braced {
                    attrs: {
                        attrs.append(&mut outer_attrs);
                        attrs
                    },
                    brace_token,
                    items,
                }),
                _ => unreachable!(),
            }
        } else {
            Self::parse_inner(input)
        }
    }
}
