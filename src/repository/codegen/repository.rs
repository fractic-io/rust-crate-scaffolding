use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::Type;

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
        let fields = generate_struct_fields(&h.fields);
        quote! {
            #[derive(::core::clone::Clone, ::core::fmt::Debug, ::serde::Serialize, ::serde::Deserialize)]
            pub struct #name {
                #(#fields),*
            }
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
        if let ValueModel::Struct { fields } = &f.input {
            let fields_ts = generate_struct_fields(fields);
            io_structs_accum.push(quote! {
                #[derive(::core::clone::Clone, ::core::fmt::Debug, ::serde::Deserialize)]
                pub struct #input_ident {
                    #(#fields_ts),*
                }
            });
        }
        // Define output struct if needed (always define for Struct, even if
        // single field).
        if let ValueModel::Struct { fields } = &f.output {
            let fields_ts = generate_struct_fields(fields);
            io_structs_accum.push(quote! {
                #[derive(::core::clone::Clone, ::core::fmt::Debug, ::serde::Serialize)]
                pub struct #output_ident {
                    #(#fields_ts),*
                }
            });
        }

        // Build input parameters for the trait method.
        let inputs_ts = build_method_inputs(&f.input);

        // Build return type for the trait method.
        let output_ts = build_method_output(&f.output, &output_ident, f.is_direct);

        // Compose method signature.
        if f.is_blocking {
            trait_methods.push(quote! {
                fn #fn_ident(&self #inputs_ts) -> #output_ts;
            });
        } else {
            trait_methods.push(quote! {
                async fn #fn_ident(&self #inputs_ts) -> #output_ts;
            });
        }
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
            let params = fields.iter().map(|f: &FieldSpec| {
                let name = &f.name;
                let ty_tokens = &f.ty_tokens;
                quote! { #name: #ty_tokens }
            });
            quote! { , #(#params),* }
        }
    }
}

fn build_method_output(
    output: &ValueModel,
    output_struct_ident: &Ident,
    is_direct: bool,
) -> TokenStream {
    fn wrap(is_direct: bool, ty: TokenStream) -> TokenStream {
        if is_direct {
            quote! { #ty }
        } else {
            quote! { ::std::result::Result<#ty, ::fractic_server_error::ServerError> }
        }
    }
    match output {
        ValueModel::None => wrap(is_direct, quote! { () }),
        ValueModel::SingleType { ty_tokens } => wrap(is_direct, quote! { #ty_tokens }),
        ValueModel::Struct { fields, .. } => {
            if fields.len() == 1 {
                let ty = &fields[0].ty_tokens;
                wrap(is_direct, quote! { #ty })
            } else {
                let out = output_struct_ident;
                wrap(is_direct, quote! { #out })
            }
        }
    }
}

fn generate_struct_fields(fields: &[FieldSpec]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f| {
            let attrs = &f.attrs;
            let name = &f.name;
            let ty = strip_top_level_reference(f.ty_tokens.clone());
            quote! { #(#attrs)* pub #name: #ty }
        })
        .collect()
}

fn strip_top_level_reference(tokens: TokenStream) -> TokenStream {
    if let Ok(ty) = syn::parse2::<Type>(tokens.clone()) {
        if let Type::Reference(r) = ty {
            let inner = *r.elem;
            return quote! { #inner };
        }
    }
    tokens
}
