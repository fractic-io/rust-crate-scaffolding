use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::{
    helpers::to_pascal_case,
    repository::model::{ConfigModel, FieldSpec, ValueModel},
};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let helper_structs = generate_helper_structs(model);
    let (io_structs, trait_methods) = generate_functions_and_trait_methods(model);
    let repo_name = &model.repository_name;

    quote! {
        #helper_structs
        #io_structs

        #[::async_trait::async_trait]
        pub trait #repo_name: ::std::marker::Send + ::std::marker::Sync {
            #(#trait_methods)*
        }
    }
}

fn generate_helper_structs(model: &ConfigModel) -> TokenStream {
    let helpers = model.helper_structs.iter().map(|h| {
        let name = &h.name;
        let body = &h.raw_tokens;
        quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            pub struct #name #body
        }
    });
    quote! { #(#helpers)* }
}

fn generate_functions_and_trait_methods(model: &ConfigModel) -> (TokenStream, Vec<TokenStream>) {
    let mut io_structs_accum = Vec::new();
    let mut trait_methods = Vec::new();

    for f in &model.functions {
        let fn_ident = &f.name;
        let fn_name = fn_ident.to_string();
        let base_pascal = to_pascal_case(&fn_name);
        let input_ident: Ident = format_ident!("{}Input", base_pascal);
        let output_ident: Ident = format_ident!("{}Output", base_pascal);

        // Define input struct if needed.
        if let ValueModel::Struct { raw_tokens, .. } = &f.input {
            let body = raw_tokens;
            io_structs_accum.push(quote! {
                #[derive(::serde::Deserialize)]
                pub struct #input_ident #body
            });
        }
        // Define output struct if needed (always define for Struct, even if
        // single field).
        if let ValueModel::Struct { raw_tokens, .. } = &f.output {
            let body = raw_tokens;
            io_structs_accum.push(quote! {
                #[derive(::serde::Serialize)]
                pub struct #output_ident #body
            });
        }

        // Build input parameters for the trait method.
        let inputs_ts = build_method_inputs(&f.input);

        // Build return type for the trait method.
        let output_ts = build_method_output(&f.output, &output_ident);

        // Compose async method signature.
        trait_methods.push(quote! {
            async fn #fn_ident(&self #inputs_ts) -> #output_ts;
        });
    }

    (quote! { #(#io_structs_accum)* }, trait_methods)
}

fn build_method_inputs(input: &ValueModel) -> TokenStream {
    match input {
        ValueModel::None => quote! {},
        ValueModel::SingleType { ty_tokens } => {
            quote! { , input: #ty_tokens }
        }
        ValueModel::Struct { fields, .. } => {
            let params = fields.iter().map(|FieldSpec { name, ty_tokens }| {
                quote! { #name: #ty_tokens }
            });
            quote! { , #(#params),* }
        }
    }
}

fn build_method_output(output: &ValueModel, output_struct_ident: &Ident) -> TokenStream {
    match output {
        ValueModel::None => {
            quote! { ::std::result::Result<(), ::fractic_server_error::ServerError> }
        }
        ValueModel::SingleType { ty_tokens } => {
            quote! { ::std::result::Result<#ty_tokens, ::fractic_server_error::ServerError> }
        }
        ValueModel::Struct { fields, .. } => {
            if fields.len() == 1 {
                let ty = &fields[0].ty_tokens;
                quote! { ::std::result::Result<#ty, ::fractic_server_error::ServerError> }
            } else {
                let out = output_struct_ident;
                quote! { ::std::result::Result<#out, ::fractic_server_error::ServerError> }
            }
        }
    }
}
