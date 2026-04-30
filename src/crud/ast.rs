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
            if ObjectKind::is_object_leading_ident(&repository_name) {
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
    Ordered,
    Unordered,
    Batch,
    Singleton,
    IndexedSingleton,
}

impl ObjectKind {
    fn expected_list() -> &'static str {
        "`root`, `ordered`, `unordered`, `batch`, `singleton`, or `indexed_singleton`"
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "root" => Some(Self::Root),
            "ordered" => Some(Self::Ordered),
            "unordered" => Some(Self::Unordered),
            "batch" => Some(Self::Batch),
            "singleton" => Some(Self::Singleton),
            "indexed_singleton" => Some(Self::IndexedSingleton),
            _ => None,
        }
    }

    fn is_object_leading_ident(ident: &Ident) -> bool {
        let ident_str = ident.to_string();
        ident_str == "archive" || Self::from_str(ident_str.as_str()).is_some()
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
    pub is_archive: bool,
    pub kind: ObjectKind,
    pub name: Ident,
    pub props: ObjectPropsRaw,
}

impl Parse for ObjectDef {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let is_archive = parse_archive_prefix(input)?;
        let kind: ObjectKind = input.parse()?;
        let name: Ident = input.parse()?;
        let content;
        let _brace_token = braced!(content in input);

        let mut parent: Option<Vec<Ident>> = None;
        let mut ordered_children: Option<Vec<Ident>> = None;
        let mut unordered_children: Option<Vec<Ident>> = None;
        let mut batch_children: Option<Vec<Ident>> = None;
        let mut singleton_children: Option<Vec<Ident>> = None;
        let mut indexed_singleton_children: Option<Vec<Ident>> = None;

        while !content.is_empty() {
            let key: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "parent" => {
                    if parent.is_some() {
                        return Err(Error::new(key.span(), "duplicate `parent` property"));
                    }
                    parent = Some(parse_ident_list(&content)?);
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
                "singleton_children" => {
                    if singleton_children.is_some() {
                        return Err(Error::new(
                            key.span(),
                            "duplicate `singleton_children` property",
                        ));
                    }
                    singleton_children = Some(parse_ident_list(&content)?);
                }
                "indexed_singleton_children" => {
                    if indexed_singleton_children.is_some() {
                        return Err(Error::new(
                            key.span(),
                            "duplicate `indexed_singleton_children` property",
                        ));
                    }
                    indexed_singleton_children = Some(parse_ident_list(&content)?);
                }
                _ => {
                    return Err(Error::new(
                        key.span(),
                        format!(
                            "unknown property `{}`; expected one of: `parent`, \
                             `ordered_children`, `unordered_children`, `batch_children`, \
                             `singleton_children`, `indexed_singleton_children`",
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
            is_archive,
            kind,
            name,
            props: ObjectPropsRaw {
                parent,
                ordered_children: ordered_children.unwrap_or_default(),
                unordered_children: unordered_children.unwrap_or_default(),
                batch_children: batch_children.unwrap_or_default(),
                singleton_children: singleton_children.unwrap_or_default(),
                indexed_singleton_children: indexed_singleton_children.unwrap_or_default(),
            },
        })
    }
}

fn parse_archive_prefix(input: ParseStream<'_>) -> Result<bool> {
    let fork = input.fork();
    if let Ok(ident) = fork.parse::<Ident>() {
        if ident.to_string() == "archive" {
            let _: Ident = input.parse()?;
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Debug, Default)]
pub struct ObjectPropsRaw {
    pub parent: Option<Vec<Ident>>,
    pub ordered_children: Vec<Ident>,
    pub unordered_children: Vec<Ident>,
    pub batch_children: Vec<Ident>,
    pub singleton_children: Vec<Ident>,
    pub indexed_singleton_children: Vec<Ident>,
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

#[cfg(test)]
mod tests {
    use super::{ConfigAst, ObjectKind};

    #[test]
    fn parses_archive_prefix_on_object_definitions() {
        let ast: ConfigAst = syn::parse_str(
            r#"
            MyRepo;
            archive ordered PersonaPrinciple {
                parent: Persona
            }
            "#,
        )
        .unwrap();

        assert_eq!(ast.objects.len(), 1);
        let object = &ast.objects[0];
        assert!(object.is_archive);
        assert_eq!(object.kind, ObjectKind::Ordered);
        assert_eq!(object.name.to_string(), "PersonaPrinciple");
    }

    #[test]
    fn treats_archive_as_reserved_object_leading_keyword() {
        let err = syn::parse_str::<ConfigAst>(
            r#"
            archive ordered PersonaPrinciple {
                parent: Persona
            }
            "#,
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("expected repository name before object definitions")
        );
    }
}
