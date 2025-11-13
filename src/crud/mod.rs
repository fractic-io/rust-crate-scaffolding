use proc_macro2::TokenStream;
use quote::quote;

mod ast;
mod codegen {
    pub mod repository;
    pub mod repository_impl;
}
mod model;

pub use ast::ConfigAst;
pub use model::ConfigModel;

pub fn generate(model: &ConfigModel) -> TokenStream {
    let tokens_repository = codegen::repository::generate(model);
    let tokens_repository_impl = codegen::repository_impl::generate(model);
    quote! {
        #tokens_repository
        #tokens_repository_impl
    }
}
