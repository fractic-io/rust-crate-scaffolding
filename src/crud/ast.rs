use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, Result, Token, braced, token};

#[derive(Debug)]
pub struct ConfigAst {
    pub repository_name: Ident,
    pub objects: Vec<ObjectDef>,
}

impl Parse for ConfigAst {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // Expect a repository name first, followed by a semicolon.
        let repository_name: Ident = input.parse()?;
        if !input.peek(Token![;]) {
            if ObjectKind::is_keyword_ident(&repository_name) {
                // Provide a more helpful error if they started with an object
                // keyword.
                return Err(Error::new(
                    repository_name.span(),
                    "expected repository name before object definitions; add an identifier and \
                     `;` (e.g., `MyRepo;`)",
                ));
            } else {
                // Otherwise a more generic error message.
                return Err(Error::new(
                    repository_name.span(),
                    "expected `;` after repository name (e.g., `MyRepo;`)",
                ));
            }
        }
        let _semi: Token![;] = input.parse()?; // Consume semicolon.

        // Parse the object definitions.
        let mut objects = Vec::new();
        while !input.is_empty() {
            objects.push(input.parse()?);
            // Optional separators (newlines/whitespace are naturally consumed
            // by syn).
        }
        Ok(Self {
            repository_name,
            objects,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Root,
    OrderedChild,
    UnorderedChild,
    BatchChild,
}

impl ObjectKind {
    fn expected_list() -> &'static str {
        "`root`, `ordered_child`, `unordered_child`, or `batch_child`"
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "root" => Some(Self::Root),
            "ordered_child" => Some(Self::OrderedChild),
            "unordered_child" => Some(Self::UnorderedChild),
            "batch_child" => Some(Self::BatchChild),
            _ => None,
        }
    }

    fn is_keyword_ident(ident: &Ident) -> bool {
        Self::from_str(ident.to_string().as_str()).is_some()
    }
}

impl Parse for ObjectKind {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ident: Ident = input.parse()?;
        if let Some(kind) = Self::from_str(ident.to_string().as_str()) {
            Ok(kind)
        } else {
            Err(Error::new(
                ident.span(),
                format!(
                    "unknown type `{}`; expected {}",
                    ident,
                    Self::expected_list()
                ),
            ))
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
        let mut ordered_children: Option<Vec<Ident>> = None;
        let mut unordered_children: Option<Vec<Ident>> = None;
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
                "ordered_children" => {
                    if ordered_children.is_some() {
                        return Err(Error::new(
                            key.span(),
                            "duplicate `ordered_children` property",
                        ));
                    }
                    ordered_children = Some(parse_ident_list(&content)?);
                }
                "unordered_children" => {
                    if unordered_children.is_some() {
                        return Err(Error::new(
                            key.span(),
                            "duplicate `unordered_children` property",
                        ));
                    }
                    unordered_children = Some(parse_ident_list(&content)?);
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
                            "unknown property `{}`; expected one of: `parent`, \
                             `ordered_children`, `unordered_children`, `batch_children`",
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
                ordered_children: ordered_children.unwrap_or_default(),
                unordered_children: unordered_children.unwrap_or_default(),
                batch_children: batch_children.unwrap_or_default(),
            },
        })
    }
}

#[derive(Debug, Default)]
pub struct ObjectPropsRaw {
    pub parent: Option<Ident>,
    pub ordered_children: Vec<Ident>,
    pub unordered_children: Vec<Ident>,
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
