use proc_macro2::TokenStream;
use quote::quote;

mod ast;
mod codegen {
    pub mod cli;
    pub mod handlers;
    pub mod repository;
}
mod model;

pub use ast::ConfigAst;
pub use model::ConfigModel;

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repository_trait = codegen::repository::generate(model);
    let handlers_macro = codegen::handlers::generate(model);
    let cli_macro = codegen::cli::generate(model);
    quote! {
        #repository_trait
        #handlers_macro
        #cli_macro
    }
}
