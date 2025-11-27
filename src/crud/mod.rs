use proc_macro2::TokenStream;
use quote::quote;

mod ast;
mod codegen {
    pub mod annotations;
    pub mod handlers;
    pub mod repository;
    pub mod repository_impl;
}
mod model;

pub use ast::ConfigAst;
pub use model::ConfigModel;

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repository_trait = codegen::repository::generate(model);
    let repository_impl_macro = codegen::repository_impl::generate(model);
    let annotations_macro = codegen::annotations::generate(model);
    let handlers_macro = codegen::handlers::generate(model);
    quote! {
        #repository_trait
        #repository_impl_macro
        #annotations_macro
        #handlers_macro
    }
}
