use syn::braced;
use syn::parse::{Parse, ParseStream};
use syn::token::Brace;

pub(crate) struct MaybeBracedItems<T> {
    pub brace_token: Option<Brace>,
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

        let mut items = Vec::new();
        while !content.is_empty() {
            items.push(input.parse()?);
        }

        Ok(Self { brace_token, items })
    }
}
