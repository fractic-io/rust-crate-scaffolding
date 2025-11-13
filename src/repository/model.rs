use syn::{Error, Result};

use crate::repository::ast;

#[derive(Debug)]
pub struct ConfigModel {}

impl TryFrom<ast::ConfigAst> for ConfigModel {
    type Error = Error;

    fn try_from(_value: ast::ConfigAst) -> Result<Self> {
        Ok(Self {})
    }
}
