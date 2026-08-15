use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens as _, format_ident, quote};
use syn::Type;

use crate::{
    helpers::{repository_contract_name, to_pascal_case, to_snake_case},
    repository::model::{ConfigModel, FieldSpec, OperationClass, ValueModel},
};

// Public interface.
// ----------------------------------------------------------------------------

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let repo_name_snake = to_snake_case(&repo_name.to_string());
    let contract_module_name =
        Ident::new(&format!("{}_contract", repo_name_snake), repo_name.span());
    let repository_name = repository_contract_name(&repo_name.to_string());
    let descriptors = model.functions.iter().map(operation_descriptor);
    let dispatch_arms = model.functions.iter().map(dispatch_arm);

    quote! {
        pub mod #contract_module_name {
            use super::*;
            use ::fractic_crate_scaffolding::contract as __contract;

            pub static DESCRIPTOR: __contract::RepositoryDescriptor =
                __contract::RepositoryDescriptor {
                    name: #repository_name,
                    repository_type: stringify!(#repo_name),
                    operations: &[#(#descriptors),*],
                };

            pub async fn dispatch(
                repository: ::std::sync::Arc<dyn #repo_name>,
                operation: &str,
                input: ::serde_json::Value,
            ) -> ::std::result::Result<
                ::serde_json::Value,
                ::fractic_server_error::ServerError,
            > {
                match operation {
                    #(#dispatch_arms),*,
                    _ => Err(__contract::unknown_operation(
                        DESCRIPTOR.name,
                        operation,
                    )),
                }
            }
        }
    }
}

// Helpers: Descriptors.
// ----------------------------------------------------------------------------

fn operation_descriptor(function: &crate::repository::model::FunctionModel) -> TokenStream {
    let name = function.name.to_string().replace('_', "-");
    let class = class_tokens(function.class);
    let deprecated = function.is_deprecated;
    let input = value_descriptor(&function.input, true);
    let output = value_descriptor(&function.output, false);

    quote! {
        __contract::OperationDescriptor {
            name: #name,
            class: #class,
            deprecated: #deprecated,
            input: #input,
            output: #output,
        }
    }
}

fn value_descriptor(value: &ValueModel, input: bool) -> TokenStream {
    match value {
        ValueModel::None => quote! {
            __contract::ValueDescriptor {
                shape: __contract::ValueShape::None,
                rust_type: "()",
                fields: &[],
            }
        },
        ValueModel::SingleType { ty_tokens } => {
            quote! {
                __contract::ValueDescriptor {
                    shape: __contract::ValueShape::Direct,
                    rust_type: stringify!(#ty_tokens),
                    fields: &[],
                }
            }
        }
        ValueModel::Struct { fields } => {
            let field_descriptors = fields.iter().map(|field| {
                let name = field.name.to_string();
                let ty = &field.ty_tokens;
                let required = field_required(field, input);
                quote! {
                    __contract::FieldDescriptor {
                        name: #name,
                        rust_type: stringify!(#ty),
                        required: #required,
                    }
                }
            });
            quote! {
                __contract::ValueDescriptor {
                    shape: __contract::ValueShape::Object,
                    rust_type: "object",
                    fields: &[#(#field_descriptors),*],
                }
            }
        }
    }
}

fn field_required(field: &FieldSpec, input: bool) -> bool {
    if !input {
        return true;
    }
    let is_option = syn::parse2::<Type>(field.ty_tokens.clone())
        .ok()
        .is_some_and(|ty| match ty {
            Type::Path(path) => path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Option"),
            _ => false,
        });
    let has_default = field.attrs.iter().any(|attribute| {
        attribute.path().is_ident("serde")
            && attribute
                .meta
                .to_token_stream()
                .to_string()
                .contains("default")
    });
    !is_option && !has_default
}

// Helpers: Dispatch.
// ----------------------------------------------------------------------------

fn dispatch_arm(function: &crate::repository::model::FunctionModel) -> TokenStream {
    let fn_ident = &function.name;
    let operation_name = fn_ident.to_string().replace('_', "-");
    let base_pascal = to_pascal_case(&fn_ident.to_string());
    let input_ident = format_ident!("{}Input", base_pascal);
    let (decode, call_args) = dispatch_input(&function.input, &input_ident);
    let invoke = if function.is_blocking {
        quote! { repository.#fn_ident(#call_args) }
    } else {
        quote! { repository.#fn_ident(#call_args).await }
    };
    let resolve = if function.is_direct {
        quote! { let __result = #invoke; }
    } else {
        quote! { let __result = #invoke?; }
    };
    let wrap = match &function.output {
        ValueModel::Struct { fields } if fields.len() == 1 => {
            let output_ident = format_ident!("{}Output", base_pascal);
            let field = &fields[0].name;
            quote! { let __result = #output_ident { #field: __result }; }
        }
        _ => quote! {},
    };

    quote! {
        #operation_name => {
            #decode
            #resolve
            #wrap
            __contract::encode_output(__result)
        }
    }
}

fn dispatch_input(value: &ValueModel, input_ident: &Ident) -> (TokenStream, TokenStream) {
    match value {
        ValueModel::None => (quote! { __contract::require_no_input(&input)?; }, quote! {}),
        ValueModel::SingleType { ty_tokens } => (
            quote! { let __input: #ty_tokens = __contract::decode_input(input)?; },
            quote! { __input },
        ),
        ValueModel::Struct { .. } => (
            quote! { let __input: #input_ident = __contract::decode_input(input)?; },
            dispatch_struct_arguments(value),
        ),
    }
}

fn dispatch_struct_arguments(value: &ValueModel) -> TokenStream {
    let ValueModel::Struct { fields } = value else {
        unreachable!("struct arguments require a struct input")
    };
    let arguments = fields.iter().map(|field| {
        let name = &field.name;
        let reference = argument_reference_mode(field.ty_tokens.clone());
        match reference {
            ReferenceMode::Borrow => quote! { &__input.#name },
            ReferenceMode::AlreadyBorrowed | ReferenceMode::Owned => quote! { __input.#name },
        }
    });
    quote! { #(#arguments),* }
}

enum ReferenceMode {
    Owned,
    Borrow,
    AlreadyBorrowed,
}

fn argument_reference_mode(tokens: TokenStream) -> ReferenceMode {
    match syn::parse2::<Type>(tokens) {
        Ok(Type::Reference(reference)) if reference.lifetime.is_some() => {
            ReferenceMode::AlreadyBorrowed
        }
        Ok(Type::Reference(_)) => ReferenceMode::Borrow,
        _ => ReferenceMode::Owned,
    }
}

fn class_tokens(class: OperationClass) -> TokenStream {
    match class {
        OperationClass::Read => quote! { __contract::OperationClass::Read },
        OperationClass::Write => quote! { __contract::OperationClass::Write },
        OperationClass::Destructive => quote! { __contract::OperationClass::Destructive },
        OperationClass::Internal => quote! { __contract::OperationClass::Internal },
    }
}
