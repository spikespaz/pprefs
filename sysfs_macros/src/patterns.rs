use syn::parse::{Parse, ParseStream};
use syn::token::Brace;
use syn::{braced, Attribute};

pub(crate) struct MaybeBracedItems<T> {
    pub brace_token: Option<Brace>,
    pub attrs: Vec<Attribute>,
    pub items: Vec<T>,
}

impl<T: Parse> Parse for MaybeBracedItems<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let look = input.lookahead1();
        let content;
        let (brace_token, content) = if look.peek(Brace) {
            (Some(braced!(content in input)), &content)
        } else {
            (None, input)
        };
        let attrs = if brace_token.is_some() {
            Attribute::parse_inner(input)?
        } else {
            Vec::new()
        };

        let mut items = Vec::new();
        while !content.is_empty() {
            items.push(input.parse()?);
        }

        Ok(Self {
            brace_token,
            attrs,
            items,
        })
    }
}
