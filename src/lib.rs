use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse as _, Parser};

mod crud;

#[proc_macro]
pub fn crud_scaffolding(input: TokenStream) -> TokenStream {
    // Parse into AST.
    let parser = crud::ConfigAst::parse;
    let ast = match parser.parse(input) {
        Ok(cfg) => cfg,
        Err(err) => return err.to_compile_error().into(),
    };

    // Validate into semantic model.
    let model = match crud::ConfigModel::try_from(ast) {
        Ok(model) => model,
        Err(err) => return err.to_compile_error().into(),
    };

    // Hand off to codegen.
    let tokens = crud::generate(&model);

    // Always emit at least one item to keep expansion positions stable.
    let out = quote! {
        #tokens
        const _: () = ();
    };
    out.into()
}
