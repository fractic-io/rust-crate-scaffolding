use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::{
    crud::model::{ConfigModel, IndexedSingletonDef, StandardDef},
    helpers::{repository_contract_name, to_snake_case},
};

// Public interface.
// ----------------------------------------------------------------------------

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let repo_name_snake = to_snake_case(&repo_name.to_string());
    let contract_module_name =
        Ident::new(&format!("{}_contract", repo_name_snake), repo_name.span());
    let handlers_macro = Ident::new(
        &format!(
            "generate_{}_handlers",
            to_snake_case(&repo_name.to_string())
        ),
        repo_name.span(),
    );
    let repository_name = repository_contract_name(&repo_name.to_string());
    let descriptors = descriptors(model);
    let dispatch_arms = dispatch_arms(model);

    quote! {
        pub mod #contract_module_name {
            use super::*;
            use ::fractic_crate_scaffolding::contract as __contract;

            super::#handlers_macro!(@contract);

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

fn descriptors(model: &ConfigModel) -> Vec<TokenStream> {
    let mut output = Vec::new();
    for object in &model.ordered_objects {
        standard_descriptors(&mut output, object, true);
    }
    for object in &model.unordered_objects {
        standard_descriptors(&mut output, object, false);
    }
    for object in &model.batch_objects {
        collection_descriptors(&mut output, &object.name, object.parents.is_some());
    }
    for object in &model.singleton_objects {
        singleton_descriptors(&mut output, &object.name, object.parents.is_some());
    }
    for object in &model.indexed_singleton_objects {
        indexed_singleton_descriptors(&mut output, object);
    }
    output
}

fn standard_descriptors(output: &mut Vec<TokenStream>, object: &StandardDef, ordered: bool) {
    let object_name = object_name(&object.name);
    let ty = &object.name;
    let has_parent = object.parents.is_some();
    push_descriptor(
        output,
        &object_name,
        "list",
        "read",
        ty,
        fields_list(has_parent),
    );
    push_descriptor(
        output,
        &object_name,
        "create",
        "write",
        ty,
        fields_create(has_parent, ordered, false),
    );
    push_descriptor(
        output,
        &object_name,
        "create-multiple",
        "write",
        ty,
        fields_create(has_parent, ordered, true),
    );
    push_descriptor(
        output,
        &object_name,
        "read",
        "read",
        ty,
        fields_item_ref(false),
    );
    push_descriptor(
        output,
        &object_name,
        "read-multiple",
        "read",
        ty,
        fields_item_ref(true),
    );
    push_descriptor(output, &object_name, "update", "write", ty, fields_update());
    push_descriptor(
        output,
        &object_name,
        "delete",
        "destructive",
        ty,
        fields_delete(false),
    );
    push_descriptor(
        output,
        &object_name,
        "delete-multiple",
        "destructive",
        ty,
        fields_delete(true),
    );
    push_descriptor(
        output,
        &object_name,
        "delete-all",
        "destructive",
        ty,
        fields_delete_all(has_parent),
    );
}

fn collection_descriptors(output: &mut Vec<TokenStream>, ty: &Ident, has_parent: bool) {
    let object_name = object_name(ty);
    push_descriptor(
        output,
        &object_name,
        "list",
        "read",
        ty,
        fields_list(has_parent),
    );
    push_descriptor(
        output,
        &object_name,
        "replace-all",
        "destructive",
        ty,
        fields_replace_all(has_parent),
    );
    push_descriptor(
        output,
        &object_name,
        "delete-all",
        "destructive",
        ty,
        fields_delete_all(has_parent),
    );
}

fn singleton_descriptors(output: &mut Vec<TokenStream>, ty: &Ident, has_parent: bool) {
    let object_name = object_name(ty);
    push_descriptor(
        output,
        &object_name,
        "read",
        "read",
        ty,
        fields_singleton_ref(has_parent),
    );
    push_descriptor(
        output,
        &object_name,
        "create",
        "write",
        ty,
        fields_create(has_parent, false, false),
    );
    push_descriptor(
        output,
        &object_name,
        "delete",
        "destructive",
        ty,
        fields_singleton_ref(has_parent),
    );
}

fn indexed_singleton_descriptors(output: &mut Vec<TokenStream>, object: &IndexedSingletonDef) {
    let object_name = object_name(&object.name);
    let ty = &object.name;
    let has_parent = object.parents.is_some();
    push_descriptor(
        output,
        &object_name,
        "list",
        "read",
        ty,
        fields_list(has_parent),
    );
    push_descriptor(
        output,
        &object_name,
        "read",
        "read",
        ty,
        fields_indexed_ref(has_parent, false),
    );
    push_descriptor(
        output,
        &object_name,
        "read-multiple",
        "read",
        ty,
        fields_indexed_ref(has_parent, true),
    );
    push_descriptor(
        output,
        &object_name,
        "create",
        "write",
        ty,
        fields_create(has_parent, false, false),
    );
    push_descriptor(
        output,
        &object_name,
        "create-multiple",
        "write",
        ty,
        fields_create(has_parent, false, true),
    );
    push_descriptor(
        output,
        &object_name,
        "delete",
        "destructive",
        ty,
        fields_indexed_ref(has_parent, false),
    );
    push_descriptor(
        output,
        &object_name,
        "delete-multiple",
        "destructive",
        ty,
        fields_indexed_ref(has_parent, true),
    );
    push_descriptor(
        output,
        &object_name,
        "delete-all",
        "destructive",
        ty,
        fields_delete_all(has_parent),
    );
}

fn push_descriptor(
    output: &mut Vec<TokenStream>,
    object: &str,
    operation: &str,
    class: &str,
    ty: &Ident,
    fields: TokenStream,
) {
    let name = format!("{object}.{operation}");
    let class = match class {
        "read" => quote! { __contract::OperationClass::Read },
        "write" => quote! { __contract::OperationClass::Write },
        _ => quote! { __contract::OperationClass::Destructive },
    };
    let output_descriptor = output_descriptor(operation, ty);
    output.push(quote! {
        __contract::OperationDescriptor {
            name: #name,
            class: #class,
            deprecated: false,
            input: __contract::ValueDescriptor {
                shape: __contract::ValueShape::Object,
                rust_type: concat!("CrudOperation<", stringify!(#ty), ">"),
                fields: #fields,
            },
            output: #output_descriptor,
        }
    });
}

fn output_descriptor(operation: &str, ty: &Ident) -> TokenStream {
    match operation {
        "list" | "read-multiple" => quote! {
            __contract::ValueDescriptor {
                shape: __contract::ValueShape::Direct,
                rust_type: concat!("Vec<", stringify!(#ty), ">"),
                fields: &[],
            }
        },
        "read" => quote! {
            __contract::ValueDescriptor {
                shape: __contract::ValueShape::Direct,
                rust_type: stringify!(#ty),
                fields: &[],
            }
        },
        "create" => quote! {
            __contract::ValueDescriptor {
                shape: __contract::ValueShape::Object,
                rust_type: "object",
                fields: &[__contract::FieldDescriptor {
                    name: "created_id",
                    rust_type: "PkSk",
                    required: true,
                }],
            }
        },
        "create-multiple" => quote! {
            __contract::ValueDescriptor {
                shape: __contract::ValueShape::Object,
                rust_type: "object",
                fields: &[__contract::FieldDescriptor {
                    name: "created_ids",
                    rust_type: "Vec<PkSk>",
                    required: true,
                }],
            }
        },
        "update" | "delete" | "delete-multiple" | "delete-all" | "replace-all" => quote! {
            __contract::ValueDescriptor {
                shape: __contract::ValueShape::None,
                rust_type: "()",
                fields: &[],
            }
        },
        _ => unreachable!("unsupported CRUD operation descriptor: {operation}"),
    }
}

// Helpers: Dispatch.
// ----------------------------------------------------------------------------

fn dispatch_arms(model: &ConfigModel) -> Vec<TokenStream> {
    let mut output = Vec::new();
    for object in model.ordered_objects.iter().chain(&model.unordered_objects) {
        operation_dispatches(
            &mut output,
            &object.name,
            &[
                "list",
                "create",
                "create-multiple",
                "read",
                "read-multiple",
                "update",
                "delete",
                "delete-multiple",
                "delete-all",
            ],
        );
    }
    for object in &model.batch_objects {
        operation_dispatches(
            &mut output,
            &object.name,
            &["list", "replace-all", "delete-all"],
        );
    }
    for object in &model.singleton_objects {
        operation_dispatches(&mut output, &object.name, &["read", "create", "delete"]);
    }
    for object in &model.indexed_singleton_objects {
        operation_dispatches(
            &mut output,
            &object.name,
            &[
                "list",
                "read",
                "read-multiple",
                "create",
                "create-multiple",
                "delete",
                "delete-multiple",
                "delete-all",
            ],
        );
    }
    output
}

fn operation_dispatches(output: &mut Vec<TokenStream>, ty: &Ident, operations: &[&str]) {
    let object = object_name(ty);
    let handler = format_ident!("manage_{}_contract_handler", to_snake_case(&ty.to_string()));
    for operation in operations {
        let name = format!("{object}.{operation}");
        output.push(quote! {
            #name => {
                let __operation: ::fractic_aws_apigateway::CrudOperation<#ty> =
                    __contract::decode_tagged_input(
                        input,
                        "operation",
                        &#operation.replace('-', "_"),
                    )?;
                let __result = #handler(repository.clone(), __operation).await?;
                __contract::encode_output(__result)
            }
        });
    }
}

fn object_name(ident: &Ident) -> String {
    to_snake_case(&ident.to_string()).replace('_', "-")
}

fn field(name: &str, ty: &str, required: bool) -> TokenStream {
    quote! { __contract::FieldDescriptor { name: #name, rust_type: #ty, required: #required } }
}

fn fields_list(has_parent: bool) -> TokenStream {
    if has_parent {
        let parent = field("parent_id", "PkSk", true);
        quote! { &[#parent] }
    } else {
        quote! { &[] }
    }
}

fn fields_create(has_parent: bool, ordered: bool, multiple: bool) -> TokenStream {
    let mut fields = Vec::new();
    if has_parent {
        fields.push(field("parent_id", "PkSk", true));
    }
    if ordered {
        fields.push(field("after", "Option<PkSk>", false));
    }
    fields.push(field(
        "data",
        if multiple { "Vec<Data>" } else { "Data" },
        true,
    ));
    quote! { &[#(#fields),*] }
}

fn fields_item_ref(multiple: bool) -> TokenStream {
    let item = field(
        if multiple { "item_refs" } else { "item_ref" },
        if multiple { "Vec<PkSk>" } else { "PkSk" },
        true,
    );
    quote! { &[#item] }
}

fn fields_update() -> TokenStream {
    let item = field("item", "Object", true);
    quote! { &[#item] }
}

fn fields_delete(multiple: bool) -> TokenStream {
    let target = field(
        if multiple { "item_refs" } else { "item_ref" },
        if multiple { "Vec<PkSk>" } else { "PkSk" },
        true,
    );
    let non_recursive = field("non_recursive", "bool", false);
    quote! { &[#target, #non_recursive] }
}

fn fields_delete_all(has_parent: bool) -> TokenStream {
    let mut fields = Vec::new();
    if has_parent {
        fields.push(field("parent_id", "PkSk", true));
    }
    fields.push(field("non_recursive", "bool", false));
    quote! { &[#(#fields),*] }
}

fn fields_replace_all(has_parent: bool) -> TokenStream {
    let mut fields = Vec::new();
    if has_parent {
        fields.push(field("parent_id", "PkSk", true));
    }
    fields.push(field("data", "Vec<Data>", true));
    quote! { &[#(#fields),*] }
}

fn fields_singleton_ref(has_parent: bool) -> TokenStream {
    let item_ref = field(
        "item_ref",
        if has_parent {
            "{ parent_id: PkSk }"
        } else {
            "{ parent_id: null }"
        },
        true,
    );
    quote! { &[#item_ref] }
}

fn fields_indexed_ref(has_parent: bool, multiple: bool) -> TokenStream {
    let target = field(
        if multiple { "item_refs" } else { "item_ref" },
        match (has_parent, multiple) {
            (true, true) => "{ parent_id: PkSk, keys: Vec<String> }",
            (true, false) => "{ parent_id: PkSk, key: String }",
            (false, true) => "{ parent_id: null, keys: Vec<String> }",
            (false, false) => "{ parent_id: null, key: String }",
        },
        true,
    );
    quote! { &[#target] }
}
