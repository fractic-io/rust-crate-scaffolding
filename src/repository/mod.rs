use proc_macro2::TokenStream;
use quote::quote;

mod ast;
mod codegen {
    pub mod placeholder;
}
mod model;

pub use ast::ConfigAst;
pub use model::ConfigModel;

pub fn generate(model: &ConfigModel) -> TokenStream {
    let placeholder_tokens = codegen::placeholder::generate(model);
    quote! {
        #placeholder_tokens
    }
}
