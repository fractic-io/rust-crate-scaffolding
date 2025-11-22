use proc_macro2::{Group, Span, TokenStream as TokenStream2, TokenTree};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Error, Ident, Result, Token, braced, token};

mod kw {
    syn::custom_keyword!(function);
    syn::custom_keyword!(function_direct);
    syn::custom_keyword!(blocking);
    syn::custom_keyword!(blocking_direct);
    syn::custom_keyword!(input);
    syn::custom_keyword!(output);
    syn::custom_keyword!(deprecated);
}

#[derive(Debug)]
pub struct ConfigAst {
    pub repository_name: Ident,
    pub functions: Vec<FunctionAst>,
}

impl Parse for ConfigAst {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // Expect repository name then semicolon.
        let repository_name: Ident = input.parse()?;
        let _semi: Token![;] = input.parse()?;

        // Parse zero or more function blocks.
        let mut functions = Vec::new();
        while !input.is_empty() {
            // Check for accidental comma.
            if input.peek(Token![,]) {
                // Consume the comma to anchor the error here, then report.
                let _comma: Token![,] = input.parse()?;
                return Err(Error::new(
                    Span::call_site(),
                    "unexpected ',' between function blocks",
                ));
            }
            functions.push(input.parse()?);
        }

        Ok(Self {
            repository_name,
            functions,
        })
    }
}

#[derive(Debug)]
pub struct FunctionAst {
    pub name: Ident,
    pub input: ValueAst,
    pub output: ValueAst,
    pub kind: FunctionKindAst,
    pub deprecated: Option<DeprecatedAst>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKindAst {
    Async,
    AsyncDirect,
    Blocking,
    BlockingDirect,
}

#[derive(Debug)]
pub enum DeprecatedAst {
    Flag,
    Note(syn::LitStr),
}

impl Parse for FunctionAst {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // 'function' | 'blocking' <name> { input: ..., output: ... }
        let kind = if input.peek(kw::function) {
            let _func_kw: kw::function = input.parse()?;
            FunctionKindAst::Async
        } else if input.peek(kw::function_direct) {
            let _func_kw: kw::function_direct = input.parse()?;
            FunctionKindAst::AsyncDirect
        } else if input.peek(kw::blocking) {
            let _blk_kw: kw::blocking = input.parse()?;
            FunctionKindAst::Blocking
        } else if input.peek(kw::blocking_direct) {
            let _blk_kw: kw::blocking_direct = input.parse()?;
            FunctionKindAst::BlockingDirect
        } else {
            return Err(Error::new(
                Span::call_site(),
                "expected `function`, `function_direct`, `blocking`, or `blocking_direct`",
            ));
        };
        let name: Ident = input.parse()?;

        let content;
        let _brace = braced!(content in input);

        // Parse properties: 'input', 'output', and optional 'deprecated'
        // (order-insensitive).
        let mut input_val: Option<ValueAst> = None;
        let mut output_val: Option<ValueAst> = None;
        let mut deprecated_val: Option<DeprecatedAst> = None;
        while !content.is_empty() {
            // Check for accidental comma.
            if content.peek(Token![,]) {
                let _comma: Token![,] = content.parse()?;
                return Err(Error::new(
                    name.span(),
                    "unexpected ',' in function body; do not add commas after definitions like \
                     `input` or `output`",
                ));
            }
            // Decide which key we have.
            if content.peek(kw::input) {
                // Parse: input: <value>
                let _k: kw::input = content.parse()?;
                let _colon: Token![:] = content.parse()?;
                let value = parse_value_until_key_or_end(
                    &content,
                    &[KeyStop::Output, KeyStop::Deprecated],
                )?;
                if input_val.is_some() {
                    return Err(Error::new(name.span(), "duplicate `input` property"));
                }
                input_val = Some(value);
            } else if content.peek(kw::output) {
                // Parse: output: <value>
                let _k: kw::output = content.parse()?;
                let _colon: Token![:] = content.parse()?;
                let value = parse_value_until_key_or_end(&content, &[KeyStop::Deprecated])?;
                if output_val.is_some() {
                    return Err(Error::new(name.span(), "duplicate `output` property"));
                }
                output_val = Some(value);
            } else if content.peek(kw::deprecated) {
                // Parse: deprecated | deprecated: "note..."
                let _k: kw::deprecated = content.parse()?;
                if content.peek(Token![:]) {
                    let _colon: Token![:] = content.parse()?;
                    // Require a string literal (normal or raw); provide a friendly error if not.
                    if content.peek(syn::Lit) {
                        // Try to parse as a string literal specifically for robust erroring.
                        let ahead = content.fork();
                        if ahead.parse::<syn::LitStr>().is_ok() {
                            let lit: syn::LitStr = content.parse()?;
                            if deprecated_val.is_some() {
                                return Err(Error::new(
                                    name.span(),
                                    "duplicate `deprecated` property",
                                ));
                            }
                            deprecated_val = Some(DeprecatedAst::Note(lit));
                        } else {
                            // It's a literal but not a string.
                            return Err(Error::new(
                                Span::call_site(),
                                "expected string literal for `deprecated` description; write \
                                 deprecated: \"reason\" or omit the colon for a flag",
                            ));
                        }
                    } else {
                        return Err(Error::new(
                            Span::call_site(),
                            "expected string literal after `deprecated:`; remember to add quotes \
                             (raw strings like r#\"...\"# are also supported)",
                        ));
                    }
                } else {
                    if deprecated_val.is_some() {
                        return Err(Error::new(name.span(), "duplicate `deprecated` property"));
                    }
                    deprecated_val = Some(DeprecatedAst::Flag);
                }
            } else {
                // Unexpected token in function body.
                let ahead: Ident = content.parse()?;
                return Err(Error::new(
                    ahead.span(),
                    format!(
                        "unknown key `{}`; expected `input`, `output`, or `deprecated`",
                        ahead
                    ),
                ));
            }
        }

        let input = input_val.ok_or_else(|| Error::new(name.span(), "missing `input` property"))?;
        let output =
            output_val.ok_or_else(|| Error::new(name.span(), "missing `output` property"))?;

        Ok(Self {
            name,
            input,
            output,
            kind,
            deprecated: deprecated_val,
        })
    }
}

#[derive(Debug)]
pub enum ValueAst {
    None,
    Struct(InlineStructAst),
    /// Any non-`None`/non-inline-struct type, captured verbatim for later
    /// processing.
    TypeTokens(TokenStream2),
}

#[derive(Debug)]
pub struct InlineStructAst {
    pub fields: Vec<FieldAst>,
}

impl Parse for InlineStructAst {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut fields = Vec::new();
        while !input.is_empty() {
            fields.push(input.parse()?);
            // Optional trailing comma separator between fields.
            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            }
        }
        Ok(Self { fields })
    }
}

#[derive(Debug)]
pub struct FieldAst {
    pub attrs: Vec<Attribute>,
    pub name: Ident,
    pub ty_tokens: TokenStream2,
}

impl Parse for FieldAst {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs: Vec<Attribute> = input.call(Attribute::parse_outer)?;
        // Check for accidental visibility modifier.
        if input.peek(Token![pub]) {
            let _pub_kw: Token![pub] = input.parse()?;
            return Err(Error::new(
                Span::call_site(),
                "visibility modifiers like `pub`, `pub(crate)`, or `pub(super)` are not allowed \
                 on inline struct fields; all fields are public by default",
            ));
        }
        let name: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let ty_tokens = read_type_tokens_until_comma_or_end(input)?;
        Ok(Self {
            attrs,
            name,
            ty_tokens,
        })
    }
}

/// Internal: distinguish where to stop when parsing a value inside the function
/// body.
enum KeyStop {
    Output,
    Deprecated,
}

/// Parse a ValueAst until either the next key (currently only `output`) or end
/// of the function body.
fn parse_value_until_key_or_end(content: ParseStream<'_>, stops: &[KeyStop]) -> Result<ValueAst> {
    // None?
    if content.peek(Ident) {
        // Use a fork to see if the next ident is the literal "None" and is not
        // followed immediately by ':'.
        let fork = content.fork();
        let ident: Ident = fork.parse()?;
        if ident == "None" {
            // Consume it on the main stream.
            let _none_ident: Ident = content.parse()?;
            return Ok(ValueAst::None);
        }
    }

    // Inline struct?
    if content.peek(token::Brace) {
        let inner;
        let _brace = braced!(inner in content);
        let inline_struct: InlineStructAst = inner.parse()?;
        return Ok(ValueAst::Struct(inline_struct));
    }

    // Otherwise, capture tokens until the next stop key or end of this function
    // body.
    let tokens = read_tokens_until_next_key_or_end(content, stops)?;
    Ok(ValueAst::TypeTokens(tokens))
}

/// Read tokens of a type position until either a top-level comma or the end of
/// the enclosing struct.
fn read_type_tokens_until_comma_or_end(input: ParseStream<'_>) -> Result<TokenStream2> {
    let mut out = TokenStream2::new();
    let mut angle = 0isize;
    while !input.is_empty() {
        if angle == 0 && input.peek(Token![,]) {
            break;
        }
        // Look ahead for end of struct body (unconsumed by this function).
        if angle == 0 && input.peek(token::Brace) {
            // This is the start of a new nested brace group in the type; we
            // should consume it entirely as a group.
            let group: Group = parse_next_group(input)?;
            out.extend([TokenTree::Group(group)]);
            continue;
        }
        if angle == 0 && input.is_empty() {
            break;
        }
        // Consume one token and update nesting counters.
        let tt: TokenTree = input.parse()?;
        match &tt {
            TokenTree::Group(_) => {
                // Treat parentheses and brackets as opaque groups; commas
                // inside them should not affect our top-level comma detection.
                // Braces are handled above via parse_next_group.
            }
            TokenTree::Punct(p) => {
                let ch = p.as_char();
                if ch == '<' {
                    angle += 1;
                } else if ch == '>' {
                    angle -= 1;
                }
            }
            _ => {}
        }
        out.extend([tt]);
    }
    Ok(out)
}

/// Read tokens until the next top-level `output:` key (if requested) or the end
/// of the function body.
fn read_tokens_until_next_key_or_end(
    content: ParseStream<'_>,
    stops: &[KeyStop],
) -> Result<TokenStream2> {
    let mut out = TokenStream2::new();
    let mut angle = 0isize;
    loop {
        if content.is_empty() {
            break;
        }
        if angle == 0 {
            // Stop on configured keys at top-level.
            if stops.iter().any(|s| matches!(s, KeyStop::Output))
                && content.peek(kw::output)
                && content.peek2(Token![:])
            {
                break;
            }
            if stops.iter().any(|s| matches!(s, KeyStop::Deprecated))
                && content.peek(kw::deprecated)
            {
                break;
            }
        }
        // Consume token while tracking nesting. Treat nested groups as opaque
        // but preserved.
        let tt: TokenTree = content.parse()?;
        match &tt {
            TokenTree::Group(_) => {
                // Treat parentheses and brackets as opaque groups. Braces are a
                // single token here.
            }
            TokenTree::Punct(p) => {
                let ch = p.as_char();
                if ch == '<' {
                    angle += 1;
                } else if ch == '>' {
                    angle -= 1;
                }
            }
            _ => {}
        }
        out.extend([tt]);
    }
    Ok(out)
}

fn parse_next_group(input: ParseStream<'_>) -> Result<Group> {
    // We know a group starts here; parse as TokenTree then assert it's a group.
    let tt: TokenTree = input.parse()?;
    if let TokenTree::Group(g) = tt {
        Ok(g)
    } else {
        Err(Error::new(
            proc_macro2::Span::call_site(),
            "expected a group",
        ))
    }
}
