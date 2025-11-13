use crate::ast;
use syn::{Error, Ident, Result};

#[derive(Debug)]
pub struct RepositoryScaffolding {
    pub repository_name: Ident,
    pub roots: Vec<RootDef>,
    pub children: Vec<ChildDef>,
    pub batches: Vec<BatchDef>,
}

#[derive(Debug)]
pub struct RootDef {
    pub name: Ident,
    pub children: Vec<Ident>,
    pub batch_children: Vec<Ident>,
}

#[derive(Debug)]
pub struct ChildDef {
    pub name: Ident,
    pub parent: Ident,
    pub children: Vec<Ident>,
    pub batch_children: Vec<Ident>,
}

#[derive(Debug)]
pub struct BatchDef {
    pub name: Ident,
    pub parent: Ident,
}

impl TryFrom<ast::Config> for RepositoryScaffolding {
    type Error = Error;

    fn try_from(value: ast::Config) -> Result<Self> {
        let mut roots = Vec::new();
        let mut children = Vec::new();
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
                        children: obj.props.children,
                        batch_children: obj.props.batch_children,
                    });
                }
                ast::ObjectKind::Child => {
                    let parent = obj.props.parent.ok_or_else(|| {
                        Error::new(obj.name.span(), "`child` requires a `parent`")
                    })?;
                    children.push(ChildDef {
                        name: obj.name,
                        parent,
                        children: obj.props.children,
                        batch_children: obj.props.batch_children,
                    });
                }
                ast::ObjectKind::Batch => {
                    let parent = obj.props.parent.ok_or_else(|| {
                        Error::new(obj.name.span(), "`batch` requires a `parent`")
                    })?;
                    // Disallow any children on batches
                    if !obj.props.children.is_empty() || !obj.props.batch_children.is_empty() {
                        return Err(Error::new(
                            obj.name.span(),
                            "`batch` cannot have `children` or `batch_children`",
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
            children,
            batches,
        })
    }
}
