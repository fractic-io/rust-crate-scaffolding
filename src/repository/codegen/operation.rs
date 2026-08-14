use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::Type;

use crate::{
    helpers::{to_pascal_case, to_snake_case},
    repository::model::{AccessClass, ConfigModel, FieldSpec, ValueModel},
};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let repo_name_snake = to_snake_case(&repo_name.to_string());
    let catalog_macro_name = Ident::new(
        &format!("generate_{}_operation_catalog", repo_name_snake),
        repo_name.span(),
    );
    let handler_macro_name = Ident::new(
        &format!("generate_{}_local_operation_handler", repo_name_snake),
        repo_name.span(),
    );
    let descriptors = model.functions.iter().map(operation_descriptor);
    let dispatch_arms = model.functions.iter().map(dispatch_arm);

    quote! {
        #[allow(unused_macros)]
        #[macro_export]
        macro_rules! #catalog_macro_name {
            ($module:ident, $operation_name:literal, $runtime:path) => {
                pub mod $module {
                    use $runtime as __runtime;

                    pub static DESCRIPTOR: __runtime::RepositoryDescriptor =
                        __runtime::RepositoryDescriptor {
                            name: $operation_name,
                            repository_type: stringify!(#repo_name),
                            operations: &[#(#descriptors),*],
                        };
                }
            };
        }

        #[allow(unused_macros)]
        #[macro_export]
        macro_rules! #handler_macro_name {
            ($module:ident, $descriptor:path, $runtime:path) => {
                pub mod $module {
                    use super::*;
                    use $runtime as __runtime;

                    pub struct LocalHandler<R: #repo_name + ?Sized> {
                        repository: ::std::sync::Arc<R>,
                    }

                    impl<R: #repo_name + ?Sized> LocalHandler<R> {
                        pub fn new(repository: ::std::sync::Arc<R>) -> Self {
                            Self { repository }
                        }
                    }

                    #[::async_trait::async_trait]
                    impl<R> __runtime::LocalOperationHandler for LocalHandler<R>
                    where
                        R: #repo_name + ?Sized + 'static,
                    {
                        async fn call(
                            &self,
                            operation: &str,
                            input: ::serde_json::Value,
                        ) -> ::std::result::Result<::serde_json::Value, __runtime::AgentError> {
                            match operation {
                                #(#dispatch_arms),*,
                                _ => Err(__runtime::AgentError::unknown_operation(
                                    ($descriptor).name,
                                    operation,
                                )),
                            }
                        }
                    }
                }
            };
        }

        #[allow(unused_imports)]
        pub(crate) use #catalog_macro_name;
        #[allow(unused_imports)]
        pub(crate) use #handler_macro_name;
    }
}

fn operation_descriptor(function: &crate::repository::model::FunctionModel) -> TokenStream {
    let name = function.name.to_string().replace('_', "-");
    let access = access_tokens(function.access);
    let deprecated = function.is_deprecated;
    let input = value_descriptor(&function.input, true);
    let output = value_descriptor(&function.output, false);

    quote! {
        __runtime::OperationDescriptor {
            name: #name,
            access: #access,
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
    let output_ident = format_ident!("{}Output", base_pascal);
    let (decode, call_args) = dispatch_input(&function.input, &input_ident);
    let invoke = if function.is_blocking {
        quote! { self.repository.#fn_ident(#call_args) }
    } else {
        quote! { self.repository.#fn_ident(#call_args).await }
    };
    let resolve = if function.is_direct {
        quote! { let __result = #invoke; }
    } else {
        quote! { let __result = #invoke?; }
    };
    let encode = match &function.output {
        ValueModel::None | ValueModel::SingleType { .. } => {
            quote! { __runtime::encode_output(__result) }
        }
        ValueModel::Struct { fields } if fields.len() == 1 => {
            let field = &fields[0].name;
            quote! { __runtime::encode_output(#output_ident { #field: __result }) }
        }
        ValueModel::Struct { .. } => quote! { __runtime::encode_output(__result) },
    };

    quote! {
        #operation_name => {
            #decode
            #resolve
            #encode
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
        ValueModel::Struct { fields } => {
            let args = fields.iter().map(|field| {
                let name = &field.name;
                if argument_needs_reference(field.ty_tokens.clone()) {
                    quote! { &__input.#name }
                } else {
                    quote! { __input.#name }
                }
            });
            (
                quote! { let __input: #input_ident = __runtime::decode_input(input)?; },
                quote! { #(#args),* },
            )
        }
    }
}

fn argument_needs_reference(tokens: TokenStream) -> bool {
    syn::parse2::<Type>(tokens)
        .ok()
        .is_some_and(|ty| matches!(ty, Type::Reference(reference) if reference.lifetime.is_none()))
}

fn access_tokens(access: AccessClass) -> TokenStream {
    match access {
        AccessClass::Read => quote! { __runtime::AccessClass::Read },
        AccessClass::Write => quote! { __runtime::AccessClass::Write },
        AccessClass::Destructive => quote! { __runtime::AccessClass::Destructive },
        AccessClass::Internal => quote! { __runtime::AccessClass::Internal },
    }
}

use quote::ToTokens as _;
