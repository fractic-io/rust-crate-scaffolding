use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse as _, Parser};

mod ast;
mod codegen;
mod model;

#[proc_macro]
pub fn crud_scaffolding(input: TokenStream) -> TokenStream {
    // Parse into AST.
    let parser = ast::Config::parse;
    let ast = match parser.parse(input) {
        Ok(cfg) => cfg,
        Err(err) => return err.to_compile_error().into(),
    };

    // Validate into semantic model.
    let model = match model::RepositoryScaffolding::try_from(ast) {
        Ok(model) => model,
        Err(err) => return err.to_compile_error().into(),
    };

    // Hand off to codegen (currently a no-op scaffold).
    let tokens = codegen::generate(&model);

    // Always emit at least one item to keep expansion positions stable.
    let out = quote! {
        #tokens
        const _: () = ();
    };
    out.into()
}
