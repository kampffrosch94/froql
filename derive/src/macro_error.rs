use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

// mostly taken from syn
pub struct MacroError {
    start: Span,
    end: Span,
    message: String,
}

impl MacroError {
    pub fn slice(arr: &[TokenTree], message: String) -> Self {
        let end = arr.last().expect(&message).span();
        let start = arr[0].span();
        Self {
            start,
            end,
            message,
        }
    }

    pub fn start_end(start: &TokenTree, end: &TokenTree, message: String) -> Self {
        let start = start.span();
        let end = end.span();
        Self {
            start,
            end,
            message,
        }
    }

    pub fn to_compile_error(&self) -> TokenStream {
        // compile_error!($message)
        TokenStream::from_iter(vec![
            TokenTree::Ident(Ident::new("compile_error", self.start)),
            TokenTree::Punct({
                let mut punct = Punct::new('!', Spacing::Alone);
                punct.set_span(self.start);
                punct
            }),
            TokenTree::Group({
                let mut group = Group::new(Delimiter::Brace, {
                    TokenStream::from_iter(vec![TokenTree::Literal({
                        let mut string = Literal::string(&self.message);
                        string.set_span(self.end);
                        string
                    })])
                });
                group.set_span(self.end);
                group
            }),
        ])
    }
}
