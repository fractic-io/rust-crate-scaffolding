use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::{
    crud::{
        codegen::handlers,
        model::{ConfigModel, IndexedSingletonDef, StandardDef},
    },
    helpers::to_snake_case,
};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let macro_name = Ident::new(
        &format!(
            "generate_{}_cli_endpoint",
            to_snake_case(&repo_name.to_string())
        ),
        repo_name.span(),
    );
    let descriptors = descriptors(model);
    let dispatch_arms = dispatch_arms(model);
    let handler_items = handlers::generate_for_cli(model);

    quote! {
        #[allow(unused_macros)]
        #[macro_export]
        macro_rules! #macro_name {
            ($module:ident, $endpoint_name:literal, $runtime:path) => {
                pub mod $module {
                    use super::*;
                    use $runtime as __runtime;

                    #handler_items

                    pub static DESCRIPTOR: __runtime::RepositoryDescriptor =
                        __runtime::RepositoryDescriptor {
                            name: $endpoint_name,
                            repository_type: stringify!(#repo_name),
                            operations: &[#(#descriptors),*],
                        };

                    pub struct Endpoint<R: #repo_name + ?Sized> {
                        repository: ::std::sync::Arc<R>,
                    }

                    impl<R: #repo_name + ?Sized> Endpoint<R> {
                        pub fn new(repository: ::std::sync::Arc<R>) -> Self {
                            Self { repository }
                        }
                    }

                    #[::async_trait::async_trait]
                    impl<R> __runtime::RepositoryEndpoint for Endpoint<R>
                    where
                        R: #repo_name + ?Sized + 'static,
                    {
                        fn descriptor(&self) -> &'static __runtime::RepositoryDescriptor {
                            &DESCRIPTOR
                        }

                        async fn call(
                            &self,
                            operation: &str,
                            input: ::serde_json::Value,
                        ) -> ::std::result::Result<::serde_json::Value, __runtime::CliEndpointError> {
                            match operation {
                                #(#dispatch_arms),*,
                                _ => Err(__runtime::CliEndpointError::unknown_operation(
                                    $endpoint_name,
                                    operation,
                                )),
                            }
                        }
                    }
                }
            };
        }

        #[allow(unused_imports)]
        pub(crate) use #macro_name;
    }
}

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
    access: &str,
    ty: &Ident,
    fields: TokenStream,
) {
    let name = format!("{object}.{operation}");
    let access = match access {
        "read" => quote! { __runtime::AccessClass::Read },
        "write" => quote! { __runtime::AccessClass::Write },
        _ => quote! { __runtime::AccessClass::Destructive },
    };
    output.push(quote! {
        __runtime::OperationDescriptor {
            name: #name,
            access: #access,
            deprecated: false,
            input: __runtime::ValueDescriptor {
                rust_type: concat!("CrudOperation<", stringify!(#ty), ">"),
                fields: #fields,
                accepts_none: false,
            },
            output: __runtime::ValueDescriptor {
                rust_type: stringify!(#ty), fields: &[], accepts_none: false,
            },
        }
    });
}

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
    let handler = format_ident!(
        "manage_{}_handler_with_repo",
        to_snake_case(&ty.to_string())
    );
    for operation in operations {
        let name = format!("{object}.{operation}");
        output.push(quote! {
            #name => {
                let __operation: ::fractic_aws_apigateway::CrudOperation<#ty> =
                    __runtime::decode_crud_input(input, #operation)?;
                let __result = #handler(self.repository.clone(), __operation).await?;
                __runtime::encode_output(__result)
            }
        });
    }
}

fn object_name(ident: &Ident) -> String {
    to_snake_case(&ident.to_string()).replace('_', "-")
}

fn field(name: &str, ty: &str, required: bool) -> TokenStream {
    quote! { __runtime::FieldDescriptor { name: #name, rust_type: #ty, required: #required } }
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
