use proc_macro2::TokenStream;
use quote::quote;

mod ast;
mod codegen {
    pub mod repository;
}
mod model;

pub use ast::ConfigAst;
pub use model::ConfigModel;

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repository_trait = codegen::repository::generate(model);
    quote! {
        #repository_trait
    }
}
