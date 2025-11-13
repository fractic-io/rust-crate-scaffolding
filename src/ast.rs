use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, Result, Token, braced, token};

#[derive(Debug)]
pub struct Config {
    pub objects: Vec<ObjectDef>,
}

impl Parse for Config {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut objects = Vec::new();
        while !input.is_empty() {
            objects.push(input.parse()?);
            // Optional separators (newlines/whitespace are naturally consumed by syn).
        }
        Ok(Self { objects })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Root,
    Child,
    Batch,
}

impl ObjectKind {
    fn expected_list() -> &'static str {
        "`root`, `child`, or `batch`"
    }
}

impl Parse for ObjectKind {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "root" => Ok(Self::Root),
            "child" => Ok(Self::Child),
            "batch" => Ok(Self::Batch),
            _ => Err(Error::new(
                ident.span(),
                format!(
                    "unknown type `{}`; expected {}",
                    ident,
                    Self::expected_list()
                ),
            )),
        }
    }
}

#[derive(Debug)]
pub struct ObjectDef {
    pub kind: ObjectKind,
    pub name: Ident,
    pub props: ObjectPropsRaw,
}

impl Parse for ObjectDef {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let kind: ObjectKind = input.parse()?;
        let name: Ident = input.parse()?;
        let content;
        let _brace_token = braced!(content in input);

        let mut parent: Option<Ident> = None;
        let mut children: Option<Vec<Ident>> = None;
        let mut batch_children: Option<Vec<Ident>> = None;

        while !content.is_empty() {
            let key: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "parent" => {
                    if parent.is_some() {
                        return Err(Error::new(key.span(), "duplicate `parent` property"));
                    }
                    let id: Ident = content.parse()?;
                    parent = Some(id);
                }
                "children" => {
                    if children.is_some() {
                        return Err(Error::new(key.span(), "duplicate `children` property"));
                    }
                    children = Some(parse_ident_list(&content)?);
                }
                "batch_children" => {
                    if batch_children.is_some() {
                        return Err(Error::new(
                            key.span(),
                            "duplicate `batch_children` property",
                        ));
                    }
                    batch_children = Some(parse_ident_list(&content)?);
                }
                _ => {
                    return Err(Error::new(
                        key.span(),
                        format!(
                            "unknown property `{}`; expected one of: `parent`, `children`, \
                             `batch_children`",
                            key
                        ),
                    ));
                }
            }

            // Optional trailing comma between properties.
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        Ok(Self {
            kind,
            name,
            props: ObjectPropsRaw {
                parent,
                children: children.unwrap_or_default(),
                batch_children: batch_children.unwrap_or_default(),
            },
        })
    }
}

#[derive(Debug, Default)]
pub struct ObjectPropsRaw {
    pub parent: Option<Ident>,
    pub children: Vec<Ident>,
    pub batch_children: Vec<Ident>,
}

fn parse_ident_list(input: ParseStream<'_>) -> Result<Vec<Ident>> {
    // Accept a single ident or a comma-separated list of idents.
    let first: Ident = input.parse()?;
    let mut items = vec![first];
    while input.peek(Token![,]) {
        let _comma: token::Comma = input.parse()?;
        // Permit trailing comma? Not in the examples; keep strict: require an ident after comma.
        if input.is_empty() {
            return Err(Error::new(
                Span::call_site(),
                "expected identifier after `,`",
            ));
        }
        let next: Ident = input.parse()?;
        items.push(next);
    }
    Ok(items)
}
