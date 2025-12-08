use proc_macro2::Span;
use syn::{Error, Ident, Result};

use crate::crud::ast;

#[derive(Debug)]
pub struct ConfigModel {
    pub repository_name: Ident,
    pub ordered_objects: Vec<CollectionDef>,
    pub unordered_objects: Vec<CollectionDef>,
    pub batch_objects: Vec<BatchDef>,
    pub singleton_objects: Vec<SingletonDef>,
    pub singleton_family_objects: Vec<SingletonFamilyDef>,
}

#[derive(Debug)]
pub struct CollectionDef {
    pub name: Ident,
    pub parents: Option<Vec<Ident>>,
    pub ordered_children: Vec<Ident>,
    pub unordered_children: Vec<Ident>,
    pub batch_children: Vec<Ident>,
    pub singleton_children: Vec<Ident>,
    pub singleton_family_children: Vec<Ident>,
}

#[derive(Debug)]
pub struct BatchDef {
    pub name: Ident,
    pub parents: Option<Vec<Ident>>,
}

#[derive(Debug)]
pub struct SingletonDef {
    pub name: Ident,
    pub parents: Option<Vec<Ident>>,
}

#[derive(Debug)]
pub struct SingletonFamilyDef {
    pub name: Ident,
    pub parents: Option<Vec<Ident>>,
}

impl TryFrom<ast::ConfigAst> for ConfigModel {
    type Error = Error;

    fn try_from(value: ast::ConfigAst) -> Result<Self> {
        let mut ordered_objects = Vec::new();
        let mut unordered_objects = Vec::new();
        let mut batch_objects = Vec::new();
        let mut singleton_objects = Vec::new();
        let mut singleton_family_objects = Vec::new();

        for obj in value.objects {
            let ast::ObjectDef { kind, name, props } = obj;
            let ast::ObjectPropsRaw {
                parent,
                ordered_children,
                unordered_children,
                batch_children,
                singleton_children,
                singleton_family_children,
            } = props;

            match kind {
                ast::ObjectKind::Root => {
                    if parent.is_some() {
                        return Err(Error::new(
                            name.span(),
                            "`root` objects cannot have a `parent` property",
                        ));
                    }
                    unordered_objects.push(CollectionDef {
                        name,
                        parents: None,
                        ordered_children,
                        unordered_children,
                        batch_children,
                        singleton_children,
                        singleton_family_children,
                    });
                }
                ast::ObjectKind::Ordered => {
                    let parents = validate_parents(name.span(), "`ordered`", parent)?;
                    ordered_objects.push(CollectionDef {
                        name,
                        parents,
                        ordered_children,
                        unordered_children,
                        batch_children,
                        singleton_children,
                        singleton_family_children,
                    });
                }
                ast::ObjectKind::Unordered => {
                    let parents = validate_parents(name.span(), "`unordered`", parent)?;
                    unordered_objects.push(CollectionDef {
                        name,
                        parents,
                        ordered_children,
                        unordered_children,
                        batch_children,
                        singleton_children,
                        singleton_family_children,
                    });
                }
                ast::ObjectKind::Batch => {
                    let parents = validate_parents(name.span(), "`batch`", parent)?;
                    if !ordered_children.is_empty()
                        || !unordered_children.is_empty()
                        || !batch_children.is_empty()
                        || !singleton_children.is_empty()
                        || !singleton_family_children.is_empty()
                    {
                        return Err(Error::new(
                            name.span(),
                            "`batch` objects cannot have `ordered_children`, \
                             `unordered_children`, `batch_children`, `singleton_children`, or \
                             `singleton_family_children`",
                        ));
                    }
                    batch_objects.push(BatchDef { name, parents });
                }
                ast::ObjectKind::Singleton => {
                    if !ordered_children.is_empty()
                        || !unordered_children.is_empty()
                        || !batch_children.is_empty()
                        || !singleton_children.is_empty()
                        || !singleton_family_children.is_empty()
                    {
                        return Err(Error::new(
                            name.span(),
                            "`singleton` objects cannot have child properties",
                        ));
                    }

                    if let Some(ref parents) = parent {
                        if parents.is_empty() {
                            return Err(Error::new(
                                name.span(),
                                "`singleton` objects require at least one `parent` when `parent` \
                                 is specified",
                            ));
                        }
                    }

                    singleton_objects.push(SingletonDef {
                        name,
                        parents: parent,
                    });
                }
                ast::ObjectKind::SingletonFamily => {
                    if !ordered_children.is_empty()
                        || !unordered_children.is_empty()
                        || !batch_children.is_empty()
                        || !singleton_children.is_empty()
                        || !singleton_family_children.is_empty()
                    {
                        return Err(Error::new(
                            name.span(),
                            "`singleton_family` objects cannot have child properties",
                        ));
                    }

                    if let Some(ref parents) = parent {
                        if parents.is_empty() {
                            return Err(Error::new(
                                name.span(),
                                "`singleton_family` objects require at least one `parent` when \
                                 `parent` is specified",
                            ));
                        }
                    }

                    singleton_family_objects.push(SingletonFamilyDef {
                        name,
                        parents: parent,
                    });
                }
            }
        }

        Ok(Self {
            repository_name: value.repository_name,
            ordered_objects,
            unordered_objects,
            batch_objects,
            singleton_objects,
            singleton_family_objects,
        })
    }
}

impl CollectionDef {
    pub fn has_children(&self) -> bool {
        !self.ordered_children.is_empty()
            || !self.unordered_children.is_empty()
            || !self.batch_children.is_empty()
            || !self.singleton_children.is_empty()
            || !self.singleton_family_children.is_empty()
    }
}

fn validate_parents(
    span: Span,
    kind_label: &str,
    parents: Option<Vec<Ident>>,
) -> Result<Option<Vec<Ident>>> {
    if let Some(ref list) = parents {
        if list.is_empty() {
            return Err(Error::new(
                span,
                format!(
                    "{} objects require at least one `parent` when `parent` is specified",
                    kind_label
                ),
            ));
        }
    }
    Ok(parents)
}
