use syn::{Error, Ident, Result};

use crate::crud::ast;

#[derive(Debug)]
pub struct ConfigModel {
    pub repository_name: Ident,
    pub roots: Vec<RootDef>,
    pub ordered_children: Vec<ChildDef>,
    pub unordered_children: Vec<ChildDef>,
    pub batches: Vec<BatchDef>,
}

#[derive(Debug)]
pub struct RootDef {
    pub name: Ident,
    pub ordered_children: Vec<Ident>,
    pub unordered_children: Vec<Ident>,
    pub batch_children: Vec<Ident>,
}

#[derive(Debug)]
pub struct ChildDef {
    pub name: Ident,
    pub parent: Ident,
    pub ordered_children: Vec<Ident>,
    pub unordered_children: Vec<Ident>,
    pub batch_children: Vec<Ident>,
}

#[derive(Debug)]
pub struct BatchDef {
    pub name: Ident,
    pub parent: Ident,
}

impl TryFrom<ast::ConfigAst> for ConfigModel {
    type Error = Error;

    fn try_from(value: ast::ConfigAst) -> Result<Self> {
        let mut roots = Vec::new();
        let mut ordered_children = Vec::new();
        let mut unordered_children = Vec::new();
        let mut batches = Vec::new();

        for obj in value.objects {
            match obj.kind {
                ast::ObjectKind::Root => {
                    if obj.props.parent.is_some() {
                        return Err(Error::new(
                            obj.name.span(),
                            "`root` must not have a `parent` property",
                        ));
                    }
                    roots.push(RootDef {
                        name: obj.name,
                        ordered_children: obj.props.ordered_children,
                        unordered_children: obj.props.unordered_children,
                        batch_children: obj.props.batch_children,
                    });
                }
                ast::ObjectKind::OrderedChild => {
                    let parent = obj.props.parent.ok_or_else(|| {
                        Error::new(obj.name.span(), "`ordered_child` requires a `parent`")
                    })?;
                    ordered_children.push(ChildDef {
                        name: obj.name,
                        parent,
                        ordered_children: obj.props.ordered_children,
                        unordered_children: obj.props.unordered_children,
                        batch_children: obj.props.batch_children,
                    });
                }
                ast::ObjectKind::UnorderedChild => {
                    let parent = obj.props.parent.ok_or_else(|| {
                        Error::new(obj.name.span(), "`unordered_child` requires a `parent`")
                    })?;
                    unordered_children.push(ChildDef {
                        name: obj.name,
                        parent,
                        ordered_children: obj.props.ordered_children,
                        unordered_children: obj.props.unordered_children,
                        batch_children: obj.props.batch_children,
                    });
                }
                ast::ObjectKind::BatchChild => {
                    let parent = obj.props.parent.ok_or_else(|| {
                        Error::new(obj.name.span(), "`batch_child` requires a `parent`")
                    })?;
                    // Disallow any children on batches
                    if !obj.props.ordered_children.is_empty()
                        || !obj.props.unordered_children.is_empty()
                        || !obj.props.batch_children.is_empty()
                    {
                        return Err(Error::new(
                            obj.name.span(),
                            "`batch_child` cannot have `ordered_children`, `unordered_children`, \
                             or `batch_children`",
                        ));
                    }
                    batches.push(BatchDef {
                        name: obj.name,
                        parent,
                    });
                }
            }
        }

        Ok(Self {
            repository_name: value.repository_name,
            roots,
            ordered_children,
            unordered_children,
            batches,
        })
    }
}
