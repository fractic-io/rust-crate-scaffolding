use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens as _, format_ident, quote};
use syn::Type;

use crate::{
    helpers::{to_pascal_case, to_snake_case},
    repository::model::{ConfigModel, FieldSpec, OperationClass, ValueModel},
};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let repo_name_snake = to_snake_case(&repo_name.to_string());
    let interface_macro_name = Ident::new(
        &format!("generate_{}_cli_interface", repo_name_snake),
        repo_name.span(),
    );
    let handlers_macro_name = Ident::new(
        &format!("generate_{}_handlers", repo_name_snake),
        repo_name.span(),
    );
    let descriptors = model.functions.iter().map(operation_descriptor);
    let dispatch_arms = model.functions.iter().map(dispatch_arm);

    quote! {
        #[allow(unused_macros)]
        #[macro_export]
        macro_rules! #interface_macro_name {
            ($module:ident, $repository_name:literal, $runtime:path, $($repo_init:tt)+) => {
                pub mod $module {
                    use super::*;
                    use ::std::sync::Arc;
                    use $runtime as __runtime;

                    $crate::#handlers_macro_name!($($repo_init)+);

                    pub static DESCRIPTOR: __runtime::RepositoryDescriptor =
                        __runtime::RepositoryDescriptor {
                            name: $repository_name,
                            repository_type: stringify!(#repo_name),
                            operations: &[#(#descriptors),*],
                        };

                    pub async fn call(
                        operation: &str,
                        input: ::serde_json::Value,
                    ) -> ::std::result::Result<::serde_json::Value, __runtime::CliError> {
                        match operation {
                            #(#dispatch_arms),*,
                            _ => Err(__runtime::CliError::unknown_operation(
                                $repository_name,
                                operation,
                            )),
                        }
                    }
                }
            };
        }

        #[allow(unused_imports)]
        pub(crate) use #interface_macro_name;
    }
}

fn operation_descriptor(function: &crate::repository::model::FunctionModel) -> TokenStream {
    let name = function.name.to_string().replace('_', "-");
    let class = class_tokens(function.class);
    let deprecated = function.is_deprecated;
    let input = value_descriptor(&function.input, true);
    let output = value_descriptor(&function.output, false);

    quote! {
        __runtime::OperationDescriptor {
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
            __runtime::ValueDescriptor { rust_type: "()", fields: &[], accepts_none: true }
        },
        ValueModel::SingleType { ty_tokens } => {
            quote! {
                __runtime::ValueDescriptor {
                    rust_type: stringify!(#ty_tokens),
                    fields: &[__runtime::FieldDescriptor {
                        name: if #input { "input" } else { "value" },
                        rust_type: stringify!(#ty_tokens),
                        required: true,
                    }],
                    accepts_none: false,
                }
            }
        }
        ValueModel::Struct { fields } => {
            let field_descriptors = fields.iter().map(|field| {
                let name = field.name.to_string();
                let ty = &field.ty_tokens;
                let required = field_required(field, input);
                quote! {
                    __runtime::FieldDescriptor {
                        name: #name,
                        rust_type: stringify!(#ty),
                        required: #required,
                    }
                }
            });
            quote! {
                __runtime::ValueDescriptor {
                    rust_type: "object",
                    fields: &[#(#field_descriptors),*],
                    accepts_none: false,
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

fn dispatch_arm(function: &crate::repository::model::FunctionModel) -> TokenStream {
    let fn_ident = &function.name;
    let operation_name = fn_ident.to_string().replace('_', "-");
    let base_pascal = to_pascal_case(&fn_ident.to_string());
    let input_ident = format_ident!("{}Input", base_pascal);
    let handler_ident = format_ident!("{}_handler", fn_ident);
    let (decode, call_args) = dispatch_input(&function.input, &input_ident);
    let invoke = if function.is_blocking {
        quote! { #handler_ident(#call_args) }
    } else {
        quote! { #handler_ident(#call_args).await }
    };
    let resolve = if function.is_direct {
        quote! { let __result = #invoke; }
    } else {
        quote! { let __result = #invoke?; }
    };

    quote! {
        #operation_name => {
            #decode
            #resolve
            __runtime::encode_output(__result)
        }
    }
}

fn dispatch_input(value: &ValueModel, input_ident: &Ident) -> (TokenStream, TokenStream) {
    match value {
        ValueModel::None => (quote! { __runtime::require_no_input(&input)?; }, quote! {}),
        ValueModel::SingleType { ty_tokens } => (
            quote! { let __input: #ty_tokens = __runtime::decode_input(input)?; },
            quote! { __input },
        ),
        ValueModel::Struct { .. } => (
            quote! { let __input: #input_ident = __runtime::decode_input(input)?; },
            quote! { __input },
        ),
    }
}

fn class_tokens(class: OperationClass) -> TokenStream {
    match class {
        OperationClass::Read => quote! { __runtime::OperationClass::Read },
        OperationClass::Write => quote! { __runtime::OperationClass::Write },
        OperationClass::Destructive => quote! { __runtime::OperationClass::Destructive },
        OperationClass::Internal => quote! { __runtime::OperationClass::Internal },
    }
}
