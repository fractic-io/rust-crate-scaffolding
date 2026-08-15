use proc_macro2::TokenStream;
use quote::quote;

mod ast;
mod codegen {
    pub mod contract;
    pub mod handlers;
    pub mod repository;
}
mod model;

pub use ast::ConfigAst;
pub use model::ConfigModel;

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repository_trait = codegen::repository::generate(model);
    let handlers_macro = codegen::handlers::generate(model);
    let contract = codegen::contract::generate(model);
    quote! {
        #repository_trait
        #handlers_macro
        #contract
    }
}
