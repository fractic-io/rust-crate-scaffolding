use syn::{Error, Ident, Result};

use crate::crud::ast;

#[derive(Debug)]
pub struct ConfigModel {
    pub repository_name: Ident,
    pub root_objects: Vec<RootDef>,
    pub ordered_objects: Vec<ChildDef>,
    pub unordered_objects: Vec<ChildDef>,
    pub batch_objects: Vec<BatchDef>,
    pub singleton_objects: Vec<SingletonDef>,
    pub singleton_family_objects: Vec<SingletonFamilyDef>,
}

#[derive(Debug)]
pub struct RootDef {
    pub name: Ident,
    pub ordered_children: Vec<Ident>,
    pub unordered_children: Vec<Ident>,
    pub batch_children: Vec<Ident>,
    pub singleton_children: Vec<Ident>,
    pub singleton_family_children: Vec<Ident>,
}

#[derive(Debug)]
pub struct ChildDef {
    pub name: Ident,
    pub parents: Vec<Ident>,
    pub ordered_children: Vec<Ident>,
    pub unordered_children: Vec<Ident>,
    pub batch_children: Vec<Ident>,
    pub singleton_children: Vec<Ident>,
    pub singleton_family_children: Vec<Ident>,
}

#[derive(Debug)]
pub struct BatchDef {
    pub name: Ident,
    pub parents: Vec<Ident>,
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
        let mut root_objects = Vec::new();
        let mut ordered_objects = Vec::new();
        let mut unordered_objects = Vec::new();
        let mut batch_objects = Vec::new();
        let mut singleton_objects = Vec::new();
        let mut singleton_family_objects = Vec::new();

        for obj in value.objects {
            match obj.kind {
                ast::ObjectKind::Root => {
                    if obj.props.parent.is_some() {
                        return Err(Error::new(
                            obj.name.span(),
                            "`root` objects cannot have a `parent` property",
                        ));
                    }
                    root_objects.push(RootDef {
                        name: obj.name,
                        ordered_children: obj.props.ordered_children,
                        unordered_children: obj.props.unordered_children,
                        batch_children: obj.props.batch_children,
                        singleton_children: obj.props.singleton_children,
                        singleton_family_children: obj.props.singleton_family_children,
                    });
                }
                ast::ObjectKind::Ordered => {
                    let parents = obj.props.parent.ok_or_else(|| {
                        Error::new(obj.name.span(), "`ordered` objects require a `parent`")
                    })?;
                    if parents.is_empty() {
                        return Err(Error::new(
                            obj.name.span(),
                            "`ordered` objects require at least one `parent`",
                        ));
                    }
                    ordered_objects.push(ChildDef {
                        name: obj.name,
                        parents,
                        ordered_children: obj.props.ordered_children,
                        unordered_children: obj.props.unordered_children,
                        batch_children: obj.props.batch_children,
                        singleton_children: obj.props.singleton_children,
                        singleton_family_children: obj.props.singleton_family_children,
                    });
                }
                ast::ObjectKind::Unordered => {
                    let parents = obj.props.parent.ok_or_else(|| {
                        Error::new(obj.name.span(), "`unordered` objects require a `parent`")
                    })?;
                    if parents.is_empty() {
                        return Err(Error::new(
                            obj.name.span(),
                            "`unordered` objects require at least one `parent`",
                        ));
                    }
                    unordered_objects.push(ChildDef {
                        name: obj.name,
                        parents,
                        ordered_children: obj.props.ordered_children,
                        unordered_children: obj.props.unordered_children,
                        batch_children: obj.props.batch_children,
                        singleton_children: obj.props.singleton_children,
                        singleton_family_children: obj.props.singleton_family_children,
                    });
                }
                ast::ObjectKind::Batch => {
                    let parents = obj.props.parent.ok_or_else(|| {
                        Error::new(obj.name.span(), "`batch` objects require a `parent`")
                    })?;
                    if parents.is_empty() {
                        return Err(Error::new(
                            obj.name.span(),
                            "`batch` objects require at least one `parent`",
                        ));
                    }
                    // Disallow any children on batch objects.
                    if !obj.props.ordered_children.is_empty()
                        || !obj.props.unordered_children.is_empty()
                        || !obj.props.batch_children.is_empty()
                        || !obj.props.singleton_children.is_empty()
                        || !obj.props.singleton_family_children.is_empty()
                    {
                        return Err(Error::new(
                            obj.name.span(),
                            "`batch` objects cannot have `ordered_children`, \
                             `unordered_children`, `batch_children`, `singleton_children`, or \
                             `singleton_family_children`",
                        ));
                    }
                    batch_objects.push(BatchDef {
                        name: obj.name,
                        parents,
                    });
                }
                ast::ObjectKind::Singleton => {
                    if !obj.props.ordered_children.is_empty()
                        || !obj.props.unordered_children.is_empty()
                        || !obj.props.batch_children.is_empty()
                        || !obj.props.singleton_children.is_empty()
                        || !obj.props.singleton_family_children.is_empty()
                    {
                        return Err(Error::new(
                            obj.name.span(),
                            "`singleton` objects cannot have child properties",
                        ));
                    }

                    if let Some(ref parents) = obj.props.parent {
                        if parents.is_empty() {
                            return Err(Error::new(
                                obj.name.span(),
                                "`singleton` objects require at least one `parent` when `parent` \
                                 is specified",
                            ));
                        }
                    }

                    singleton_objects.push(SingletonDef {
                        name: obj.name,
                        parents: obj.props.parent,
                    });
                }
                ast::ObjectKind::SingletonFamily => {
                    if !obj.props.ordered_children.is_empty()
                        || !obj.props.unordered_children.is_empty()
                        || !obj.props.batch_children.is_empty()
                        || !obj.props.singleton_children.is_empty()
                        || !obj.props.singleton_family_children.is_empty()
                    {
                        return Err(Error::new(
                            obj.name.span(),
                            "`singleton_family` objects cannot have child properties",
                        ));
                    }

                    if let Some(ref parents) = obj.props.parent {
                        if parents.is_empty() {
                            return Err(Error::new(
                                obj.name.span(),
                                "`singleton_family` objects require at least one `parent` when \
                                 `parent` is specified",
                            ));
                        }
                    }

                    singleton_family_objects.push(SingletonFamilyDef {
                        name: obj.name,
                        parents: obj.props.parent,
                    });
                }
            }
        }

        Ok(Self {
            repository_name: value.repository_name,
            root_objects,
            ordered_objects,
            unordered_objects,
            batch_objects,
            singleton_objects,
            singleton_family_objects,
        })
    }
}

impl RootDef {
    pub fn has_children(&self) -> bool {
        !self.ordered_children.is_empty()
            || !self.unordered_children.is_empty()
            || !self.batch_children.is_empty()
            || !self.singleton_children.is_empty()
            || !self.singleton_family_children.is_empty()
    }
}

impl ChildDef {
    pub fn has_children(&self) -> bool {
        !self.ordered_children.is_empty()
            || !self.unordered_children.is_empty()
            || !self.batch_children.is_empty()
            || !self.singleton_children.is_empty()
            || !self.singleton_family_children.is_empty()
    }
}
