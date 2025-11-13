use syn::Result;
use syn::parse::{Parse, ParseStream};

#[derive(Debug)]
pub struct ConfigAst {}

impl Parse for ConfigAst {
    fn parse(_input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {})
    }
}
