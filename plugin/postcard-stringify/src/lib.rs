//! Compile-time [`postcard`](https://crates.io/crates/postcard)-like serialization.
//!
//! Note that this can't execute custom serialization/deserialization code.
//!
//! Here, take a look:
//! ```rs
//! const THIS_IS_A_CONSTANT: &[u8] = &postcard_stringify::postcard!("str" 1200 [1, 2, 3] (2, 3, 4))
//! let this_is_not_a_constant: Vec<u8> = postcard::to_allocvec(&("str", 1200, [1, 2, 3], (2, 3, 4));
//! assert_eq!(THIS_IS_A_CONSTANT, &this_is_not_a_constant);
//! ```

extern crate proc_macro;
use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

#[proc_macro]
pub fn postcard(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ts = expand_macros_in_token_stream(ts.into());

    let mut sink = Vec::new();
    token_stream_to_postcard(ts, &mut sink);
    TokenStream::from_iter([
        TokenTree::Punct(Punct::new('*', Spacing::Alone)),
        TokenTree::Literal(Literal::byte_string(&sink)),
    ])
    .into()
}

#[proc_macro]
pub fn declare(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ts = expand_macros_in_token_stream(ts.into());

    // if this returns None the syntax is invalid so just output the original token stream
    fn declare_inner(ts: TokenStream) -> Option<TokenStream> {
        let mut tokens_at_the_start = Vec::new();

        let mut iter = ts.into_iter().peekable();

        let decl_type = loop {
            match iter.next()? {
                TokenTree::Ident(ident)
                    if matches!(ident.to_string().as_str(), "static" | "const") =>
                {
                    break ident;
                }
                other => tokens_at_the_start.push(other),
            }
        };
        let name = match iter.next()? {
            TokenTree::Ident(ident) => ident,
            _ => return None,
        };
        let equals = match iter.next()? {
            TokenTree::Punct(punct) if punct.as_char() == ':' => {
                panic!("can't use type in postcard_stringify::declare! as it will be replaced, just remove the type");
            }
            TokenTree::Punct(punct) if punct.as_char() == '=' => punct,
            _ => return None,
        };

        let mut tokens: Vec<TokenTree> = iter.collect();

        let semicolon_at_the_end = match tokens.pop() {
            Some(TokenTree::Punct(punct)) if punct.as_char() == ';' => Some(punct),
            Some(other) => {
                tokens.push(other);
                None
            }
            None => None,
        };

        let mut sink = Vec::new();
        token_stream_to_postcard(TokenStream::from_iter(tokens), &mut sink);

        TokenStream::from_iter(
            tokens_at_the_start
                .into_iter()
                .chain([
                    decl_type.into(),
                    name.into(),
                    TokenTree::Punct(Punct::new(':', Spacing::Alone)),
                    TokenTree::Group(Group::new(
                        Delimiter::Bracket,
                        TokenStream::from_iter([
                            TokenTree::Ident(Ident::new("u8", Span::call_site())),
                            TokenTree::Punct(Punct::new(';', Spacing::Alone)),
                            TokenTree::Literal(Literal::usize_unsuffixed(sink.len())),
                        ]),
                    )),
                    equals.into(),
                    TokenTree::Punct(Punct::new('*', Spacing::Alone)),
                    TokenTree::Literal(Literal::byte_string(&sink)),
                ])
                .chain(semicolon_at_the_end.map(Into::into)),
        )
        .into()
    }
    match declare_inner(ts.clone()) {
        Some(stream) => stream,
        None => ts,
    }
    .into()
}

/// Rudimentary processing for builtin Rust macros.
/// e.g. `stringify!`..., uh, that's it. Maybe `concat!` and friends in the future.
fn expand_macros_in_token_stream(ts: TokenStream) -> TokenStream {
    let mut processed_tokens = Vec::new();

    enum State {
        NoMacro,
        Ident(Ident),
        MacroInvokation(String),
    }
    let mut state: State = State::NoMacro;
    for token in ts {
        match state {
            State::NoMacro => match token {
                TokenTree::Ident(ident) => {
                    state = State::Ident(ident);
                }
                TokenTree::Group(group) => {
                    processed_tokens.push(TokenTree::Group(Group::new(
                        group.delimiter(),
                        expand_macros_in_token_stream(group.stream()),
                    )));
                }
                other => processed_tokens.push(other),
            },
            State::Ident(state_ident) => {
                match token {
                    TokenTree::Punct(punct) if punct.as_char() == '!' => {
                        state = State::MacroInvokation(state_ident.to_string())
                    }
                    TokenTree::Punct(punct) => {
                        processed_tokens.push(state_ident.into());
                        processed_tokens.push(punct.into());
                        state = State::NoMacro;
                    }
                    TokenTree::Group(_group) => {
                        panic!("tried to call function {state_ident}. function calls are not allowed here");
                    }
                    TokenTree::Ident(ident) => {
                        processed_tokens.push(state_ident.into());
                        state = State::Ident(ident);
                    }
                    TokenTree::Literal(literal) => {
                        processed_tokens.push(state_ident.into());
                        processed_tokens.push(literal.into());
                        state = State::NoMacro;
                    }
                }
            }
            State::MacroInvokation(macro_name) => match token {
                TokenTree::Group(group) => {
                    match macro_name.as_str() {
                        "stringify" => processed_tokens.push(TokenTree::Literal(Literal::string(
                            &group.stream().to_string(),
                        ))),
                        other => panic!("unknown macro {other:?}"),
                    }
                    state = State::NoMacro;
                }
                _ => panic!("macro {macro_name} called without ()[]{{}} these things"),
            },
        }
    }
    if let State::Ident(ident) = state {
        processed_tokens.push(TokenTree::from(ident));
    }
    TokenStream::from_iter(processed_tokens)
}

/// Converts a [`proc_macro::TokenStream`] to a postcard data structure.
/// Returns the number of items inside the token stream, as in the case of an array.
fn token_stream_to_postcard(ts: TokenStream, sink: &mut Vec<u8>) -> usize {
    let mut num_items = 0;
    let mut is_on_item_boundary = true;
    for tt in ts {
        match tt {
            TokenTree::Punct(punct)
                if punct.spacing() == Spacing::Alone && punct.as_char() == ',' =>
            {
                is_on_item_boundary = true;
            }
            tt => {
                if is_on_item_boundary {
                    is_on_item_boundary = false;
                    num_items += 1;
                } else {
                    panic!("adjacent values aren't allowed. if you want to put them one after another, separate them with a comma");
                }
                token_tree_to_postcard(tt, sink);
            }
        }
    }
    num_items
}
fn token_tree_to_postcard(tt: TokenTree, sink: &mut Vec<u8>) {
    match tt {
        TokenTree::Group(group) => {
            match group.delimiter() {
                Delimiter::Brace => panic!("dictionaries not implemented yet, sorry!"),
                Delimiter::None | Delimiter::Parenthesis => {
                    token_stream_to_postcard(group.stream(), sink);
                }
                Delimiter::Bracket => {
                    let length_index = sink.len();

                    // first put the stuff in the vec
                    let num_items = token_stream_to_postcard(group.stream(), sink);

                    // then encode length to leb128
                    let mut buf = [0u8; 10];
                    let mut writable = &mut buf[..];
                    leb128::write::unsigned(&mut writable, num_items as u64).expect("unreachable");
                    let writable_len = writable.len();
                    let bytes = &buf[..buf.len() - writable_len];

                    // finally insert the bytes into the vec at the specified position
                    sink.splice(length_index..length_index, bytes.iter().cloned());
                }
            }
        }
        TokenTree::Ident(ident) => {
            panic!("unexpected identifier {ident}");
        }
        TokenTree::Punct(punct) => {
            panic!("unexpected {punct}");
        }
        TokenTree::Literal(literal) => {
            use syn::Lit;
            match Lit::new(literal.into()) {
                Lit::Str(str) => {
                    let string = str.value();
                    leb128::write::unsigned(sink, string.len() as u64).expect("unreachable");
                    sink.extend(string.bytes());
                }
                Lit::Int(int) => {
                    let digits = int.base10_digits();
                    if &digits[0..1] == "-" {
                        let i64: i64 = int
                            .base10_parse()
                            .unwrap_or_else(|_| panic!("number {} doesn't fit in an i64", digits));
                        leb128::write::signed(sink, i64).expect("unreachable");
                    } else {
                        let u64: u64 = int
                            .base10_parse()
                            .unwrap_or_else(|_| panic!("number {} doesn't fit in an u64", digits));
                        leb128::write::unsigned(sink, u64).expect("unreachable");
                    }
                }
                _other => panic!("unknown literal"),
            }
        }
    }
}
