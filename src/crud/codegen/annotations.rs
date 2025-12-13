//! NOTE: These annotations currently include support for root ordered / batch
//! items, even though fractic-aws-dynamo does not yet actually have manager
//! types (CRUD wrappers) for these (for ex. ManageRootOrdered does not exist).

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{
    crud::model::{
        BatchDef, ConfigModel, HasParents, SingletonDef, SingletonFamilyDef, StandardDef,
    },
    helpers::to_snake_case,
};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let repo_name_snake = to_snake_case(&repo_name.to_string());
    let macro_name_ident = Ident::new(
        &format!("generate_{}_annotations", repo_name_snake),
        repo_name.span(),
    );

    let root_items: Vec<TokenStream> = model
        .ordered_objects
        .iter()
        .filter(|root| root.parents.is_none())
        .map(|root| gen_root_standard_item(root, true))
        .chain(
            model
                .unordered_objects
                .iter()
                .filter(|root| root.parents.is_none())
                .map(|root| gen_root_standard_item(root, false)),
        )
        .chain(
            model
                .batch_objects
                .iter()
                .filter(|batch| batch.parents.is_none())
                .map(gen_root_batch_item),
        )
        .chain(
            model
                .singleton_objects
                .iter()
                .filter(|singleton| singleton.parents.is_none())
                .map(gen_root_singleton_item),
        )
        .chain(
            model
                .singleton_family_objects
                .iter()
                .filter(|singleton_family| singleton_family.parents.is_none())
                .map(gen_root_singleton_family_item),
        )
        .collect();

    let child_items: Vec<TokenStream> = {
        fn parent_of(child: &impl HasParents) -> &Ident {
            // The parent ident is used only to create dummy objects for unchecked
            // methods. Since the dummy object is only needed to satisfy the type
            // system, we can use any valid parent type (so use the first one).
            child
                .parents()
                .and_then(|p| p.first())
                .expect("child items should be verified to have at least one parent")
        }
        model
            .ordered_objects
            .iter()
            .filter(|child| child.parents.is_some())
            .map(|child| gen_child_standard_item(child, parent_of(child), true))
            .chain(
                model
                    .unordered_objects
                    .iter()
                    .filter(|child| child.parents.is_some())
                    .map(|child| gen_child_standard_item(child, parent_of(child), false)),
            )
            .chain(
                model
                    .batch_objects
                    .iter()
                    .filter(|child| child.parents.is_some())
                    .map(|child| gen_child_batch_item(child, parent_of(child))),
            )
            .chain(
                model
                    .singleton_objects
                    .iter()
                    .filter(|child| child.parents.is_some())
                    .map(|child| gen_child_singleton_item(child, parent_of(child))),
            )
            .chain(
                model
                    .singleton_family_objects
                    .iter()
                    .filter(|child| child.parents.is_some())
                    .map(|child| gen_child_singleton_family_item(child, parent_of(child))),
            )
            .collect()
    };

    let root_items_clone = root_items.clone();
    let child_items_clone = child_items.clone();

    quote! {
        #[allow(unused_macros)]
        #[macro_export]
        macro_rules! #macro_name_ident {
            (dyn $ctx_view:path => $ctx_repo_accessor:ident) => {
                // Local helper for the context type when a trait is provided.
                macro_rules! __ctx { () => { &impl $ctx_view } }
                // Roots:
                #(#root_items)*
                // Children:
                #(#child_items)*
            };
            ($ctx:ty => $ctx_repo_accessor:ident) => {
                // Local helper for the context type when a concrete type is
                // provided.
                macro_rules! __ctx { () => { & $ctx } }
                // Roots:
                #(#root_items_clone)*
                // Children:
                #(#child_items_clone)*
            };
        }

        #[allow(unused_imports)]
        pub(crate) use #macro_name_ident;
    }
}

fn gen_root_standard_item(root: &StandardDef, is_ordered: bool) -> TokenStream {
    let ty_ident = &root.name;
    let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
    let manager_ident = method_ident_for("manage", &root.name);

    let (basic_methods, basic_impls) = (
        quote! {
            async fn get(ctx: __ctx!(), id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
            async fn update(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
            async fn list(ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
        },
        quote! {
            async fn get(ctx: __ctx!(), id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                ctx.$ctx_repo_accessor().await?.#manager_ident().get(id).await
            }
            async fn update(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                ctx.$ctx_repo_accessor().await?.#manager_ident().update(self).await
            }
            async fn list(ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                ctx.$ctx_repo_accessor().await?.#manager_ident().query_all().await
            }
        },
    );

    let (add_methods, add_impls) = if is_ordered {
        (
            quote! {
                async fn add(ctx: __ctx!(), data: #ty_data_ident, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                async fn batch_add(ctx: __ctx!(), data: ::std::vec::Vec<#ty_data_ident>, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
            },
            quote! {
                async fn add(ctx: __ctx!(), data: #ty_data_ident, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().add(data, after).await
                }
                async fn batch_add(ctx: __ctx!(), data: ::std::vec::Vec<#ty_data_ident>, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().batch_add(data, after).await
                }
            },
        )
    } else {
        (
            quote! {
                async fn add(ctx: __ctx!(), data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                async fn batch_add(ctx: __ctx!(), data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
            },
            quote! {
                async fn add(ctx: __ctx!(), data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().add(data).await
                }
                async fn batch_add(ctx: __ctx!(), data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().batch_add(data).await
                }
            },
        )
    };

    let (delete_methods, delete_impls) = if root.has_children() {
        (
            quote! {
                async fn delete_recursive(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                #[allow(non_snake_case)]
                async fn delete_non_recursive_DANGEROUS(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                #[allow(non_snake_case)]
                async fn batch_delete_non_recursive_DANGEROUS(ctx: __ctx!(), items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError>;
                #[allow(non_snake_case)]
                async fn batch_delete_all_non_recursive_DANGEROUS(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
            },
            quote! {
                async fn delete_recursive(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().delete_recursive(self).await
                }
                #[allow(non_snake_case)]
                async fn delete_non_recursive_DANGEROUS(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().delete_non_recursive(self).await
                }
                #[allow(non_snake_case)]
                async fn batch_delete_non_recursive_DANGEROUS(ctx: __ctx!(), items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_non_recursive(items).await
                }
                #[allow(non_snake_case)]
                async fn batch_delete_all_non_recursive_DANGEROUS(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_all_non_recursive().await
                }
            },
        )
    } else {
        (
            quote! {
                async fn delete(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                async fn batch_delete(ctx: __ctx!(), items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError>;
                async fn batch_delete_all(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
            },
            quote! {
                async fn delete(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().delete(self).await
                }
                async fn batch_delete(ctx: __ctx!(), items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete(items).await
                }
                async fn batch_delete_all(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_all().await
                }
            },
        )
    };

    let (ordered_child_methods, ordered_child_impls) =
        root.ordered_children.iter().map(|child_name| {
            let child_ident = child_name;
            let child_data_ident = Ident::new(&format!("{}Data", child_ident), child_ident.span());
            let child_manager_ident = method_ident_for("manage", child_ident);
            let base_pascal = stripped_pascal(ty_ident, child_ident);
            let child_singular_snake = to_snake_case(&base_pascal);
            let child_plural_snake = to_snake_case(&pluralize_pascal(&base_pascal));
            let add_child_fn = Ident::new(&format!("add_{}", child_singular_snake), child_ident.span());
            let batch_add_children_fn =
                Ident::new(&format!("batch_add_{}", child_plural_snake), child_ident.span());
            let list_children_fn =
                Ident::new(&format!("list_{}", child_plural_snake), child_ident.span());
            (
                quote! {
                    async fn #add_child_fn(&self, ctx: __ctx!(), data: #child_data_ident, after: ::std::option::Option<& #child_ident>) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError>;
                    async fn #batch_add_children_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#child_data_ident>, after: ::std::option::Option<& #child_ident>) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError>;
                    async fn #list_children_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #add_child_fn(&self, ctx: __ctx!(), data: #child_data_ident, after: ::std::option::Option<& #child_ident>) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().add(self, data, after).await
                    }
                    async fn #batch_add_children_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#child_data_ident>, after: ::std::option::Option<& #child_ident>) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().batch_add(self, data, after).await
                    }
                    async fn #list_children_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().query_all(self).await
                    }
                },
            )
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    let (unordered_child_methods, unordered_child_impls) =
        root.unordered_children.iter().map(|child_name| {
            let child_ident = child_name;
            let child_data_ident = Ident::new(&format!("{}Data", child_ident), child_ident.span());
            let child_manager_ident = method_ident_for("manage", child_ident);
            let base_pascal = stripped_pascal(ty_ident, child_ident);
            let child_singular_snake = to_snake_case(&base_pascal);
            let child_plural_snake = to_snake_case(&pluralize_pascal(&base_pascal));
            let add_child_fn = Ident::new(&format!("add_{}", child_singular_snake), child_ident.span());
            let batch_add_children_fn =
                Ident::new(&format!("batch_add_{}", child_plural_snake), child_ident.span());
            let list_children_fn =
                Ident::new(&format!("list_{}", child_plural_snake), child_ident.span());
            (
                quote! {
                    async fn #add_child_fn(&self, ctx: __ctx!(), data: #child_data_ident) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError>;
                    async fn #batch_add_children_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#child_data_ident>) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError>;
                    async fn #list_children_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #add_child_fn(&self, ctx: __ctx!(), data: #child_data_ident) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().add(self, data).await
                    }
                    async fn #batch_add_children_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#child_data_ident>) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().batch_add(self, data).await
                    }
                    async fn #list_children_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().query_all(self).await
                    }
                },
            )
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    let (batch_methods, batch_impls) = root.batch_children.iter().map(|batch_name| {
        let batch_ident = batch_name;
        let batch_data_ident = Ident::new(&format!("{}Data", batch_ident), batch_ident.span());
        let batch_manager_ident = method_ident_for("manage", batch_ident);
        let base_pascal = stripped_pascal(ty_ident, batch_ident);
        let plural_snake = to_snake_case(&pluralize_pascal(&base_pascal));
        let list_fn = Ident::new(&format!("list_{}", plural_snake), batch_ident.span());
        let del_all_fn = Ident::new(&format!("batch_delete_all_{}", plural_snake), batch_ident.span());
        let replace_all_fn =
            Ident::new(&format!("batch_replace_all_{}", plural_snake), batch_ident.span());
        (
            quote! {
                async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#batch_ident>, ::fractic_server_error::ServerError>;
                async fn #del_all_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                async fn #replace_all_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#batch_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
            },
            quote! {
                async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#batch_ident>, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#batch_manager_ident().query_all(self).await
                }
                async fn #del_all_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#batch_manager_ident().batch_delete_all(self).await
                }
                async fn #replace_all_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#batch_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#batch_manager_ident().batch_replace_all_ordered(self, data).await
                }
            },
        )
    }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    let (singleton_child_methods, singleton_child_impls) = root
        .singleton_children
        .iter()
        .map(|child_name| {
            let child_ident = child_name;
            let child_data_ident = Ident::new(&format!("{}Data", child_ident), child_ident.span());
            let child_manager_ident = method_ident_for("manage", child_ident);
            let base_pascal = stripped_pascal(ty_ident, child_ident);
            let singular_snake = to_snake_case(&base_pascal);
            let get_fn = Ident::new(&format!("get_{}", singular_snake), child_ident.span());
            let set_fn = Ident::new(&format!("set_{}", singular_snake), child_ident.span());
            let delete_fn = Ident::new(&format!("delete_{}", singular_snake), child_ident.span());
            (
                quote! {
                    async fn #get_fn(&self, ctx: __ctx!()) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError>;
                    async fn #set_fn(&self, ctx: __ctx!(), data: #child_data_ident) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError>;
                    async fn #delete_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #get_fn(&self, ctx: __ctx!()) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().get(self).await
                    }
                    async fn #set_fn(&self, ctx: __ctx!(), data: #child_data_ident) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().set(self, data).await
                    }
                    async fn #delete_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().delete(self).await
                    }
                },
            )
        })
        .unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    let (singleton_family_child_methods, singleton_family_child_impls) = root
        .singleton_family_children
        .iter()
        .map(|child_name| {
            let child_ident = child_name;
            let child_data_ident = Ident::new(&format!("{}Data", child_ident), child_ident.span());
            let child_manager_ident = method_ident_for("manage", child_ident);
            let base_pascal = stripped_pascal(ty_ident, child_ident);
            let singular_snake = to_snake_case(&base_pascal);
            let plural_snake = to_snake_case(&pluralize_pascal(&base_pascal));

            let get_fn = Ident::new(&format!("get_{}", singular_snake), child_ident.span());
            let set_fn = Ident::new(&format!("set_{}", singular_snake), child_ident.span());
            let batch_set_fn = Ident::new(&format!("batch_set_{}", plural_snake), child_ident.span());
            let delete_fn = Ident::new(&format!("delete_{}", singular_snake), child_ident.span());
            let batch_delete_fn =
                Ident::new(&format!("batch_delete_{}", plural_snake), child_ident.span());
            let list_fn = Ident::new(&format!("list_{}", plural_snake), child_ident.span());
            let batch_delete_all_fn =
                Ident::new(&format!("batch_delete_all_{}", plural_snake), child_ident.span());

            (
                quote! {
                    async fn #get_fn(&self, ctx: __ctx!(), key: &str) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError>;
                    async fn #set_fn(&self, ctx: __ctx!(), data: #child_data_ident) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError>;
                    async fn #batch_set_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#child_data_ident>) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError>;
                    async fn #delete_fn(&self, ctx: __ctx!(), key: &str) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                    async fn #batch_delete_fn(&self, ctx: __ctx!(), keys: ::std::vec::Vec<&str>) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                    async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError>;
                    async fn #batch_delete_all_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #get_fn(&self, ctx: __ctx!(), key: &str) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().get(self, key).await
                    }
                    async fn #set_fn(&self, ctx: __ctx!(), data: #child_data_ident) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().set(self, data).await
                    }
                    async fn #batch_set_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#child_data_ident>) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().batch_set(self, data).await
                    }
                    async fn #delete_fn(&self, ctx: __ctx!(), key: &str) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().delete(self, key).await
                    }
                    async fn #batch_delete_fn(&self, ctx: __ctx!(), keys: ::std::vec::Vec<&str>) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().batch_delete(self, keys).await
                    }
                    async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().query_all(self).await
                    }
                    async fn #batch_delete_all_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().batch_delete_all(self).await
                    }
                },
            )
        })
        .unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

    quote! {
        pub trait #trait_ident {
            #basic_methods
            #add_methods
            #delete_methods
            #(#ordered_child_methods)*
            #(#unordered_child_methods)*
            #(#batch_methods)*
            #(#singleton_child_methods)*
            #(#singleton_family_child_methods)*
        }
        impl #trait_ident for #ty_ident {
            #basic_impls
            #add_impls
            #delete_impls
            #(#ordered_child_impls)*
            #(#unordered_child_impls)*
            #(#batch_impls)*
            #(#singleton_child_impls)*
            #(#singleton_family_child_impls)*
        }
    }
}

fn gen_root_batch_item(batch: &BatchDef) -> TokenStream {
    let ty_ident = &batch.name;
    let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
    let manager_ident = method_ident_for("manage", ty_ident);
    let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

    let methods = quote! {
        async fn list(ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
        async fn batch_delete_all(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
        async fn batch_replace_all(ctx: __ctx!(), data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
    };
    let impls = quote! {
        async fn list(ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().query_all().await
        }
        async fn batch_delete_all(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_all().await
        }
        async fn batch_replace_all(ctx: __ctx!(), data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().batch_replace_all_ordered(data).await
        }
    };

    quote! {
        pub trait #trait_ident {
            #methods
        }
        impl #trait_ident for #ty_ident {
            #impls
        }
    }
}

fn gen_child_batch_item(batch: &BatchDef, parent_ident: &Ident) -> TokenStream {
    let ty_ident = &batch.name;
    let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
    let parent_data_ident = Ident::new(&format!("{}Data", parent_ident), parent_ident.span());
    let manager_ident = method_ident_for("manage", ty_ident);
    let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

    let methods = quote! {
        async fn unchecked_list(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
        async fn unchecked_batch_delete_all(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
        async fn unchecked_batch_replace_all(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
    };
    let impls = quote! {
        async fn unchecked_list(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().query_all(&tmp_dummy).await
        }
        async fn unchecked_batch_delete_all(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_all(&tmp_dummy).await
        }
        async fn unchecked_batch_replace_all(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().batch_replace_all_ordered(&tmp_dummy, data).await
        }
    };

    quote! {
        pub trait #trait_ident {
            #methods
        }
        impl #trait_ident for #ty_ident {
            #impls
        }
    }
}

fn gen_root_singleton_item(singleton: &SingletonDef) -> TokenStream {
    let ty_ident = &singleton.name;
    let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
    let manager_ident = method_ident_for("manage", ty_ident);
    let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

    let methods = quote! {
        async fn get(ctx: __ctx!()) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
        async fn set(ctx: __ctx!(), data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
        async fn delete(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
    };
    let impls = quote! {
        async fn get(ctx: __ctx!()) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().get().await
        }
        async fn set(ctx: __ctx!(), data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().set(data).await
        }
        async fn delete(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().delete().await
        }
    };

    quote! {
        pub trait #trait_ident {
            #methods
        }
        impl #trait_ident for #ty_ident {
            #impls
        }
    }
}

fn gen_root_singleton_family_item(singleton_family: &SingletonFamilyDef) -> TokenStream {
    let ty_ident = &singleton_family.name;
    let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
    let manager_ident = method_ident_for("manage", ty_ident);
    let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

    let methods = quote! {
        async fn get(ctx: __ctx!(), key: &str) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
        async fn set(ctx: __ctx!(), data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
        async fn batch_set(ctx: __ctx!(), data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
        async fn delete(ctx: __ctx!(), key: &str) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
        async fn batch_delete(ctx: __ctx!(), keys: ::std::vec::Vec<&str>) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
        async fn list(ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
        async fn batch_delete_all(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
    };
    let impls = quote! {
        async fn get(ctx: __ctx!(), key: &str) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().get(key).await
        }
        async fn set(ctx: __ctx!(), data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().set(data).await
        }
        async fn batch_set(ctx: __ctx!(), data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().batch_set(data).await
        }
        async fn delete(ctx: __ctx!(), key: &str) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().delete(key).await
        }
        async fn batch_delete(ctx: __ctx!(), keys: ::std::vec::Vec<&str>) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete(keys).await
        }
        async fn list(ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().query_all().await
        }
        async fn batch_delete_all(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_all().await
        }
    };

    quote! {
        pub trait #trait_ident {
            #methods
        }
        impl #trait_ident for #ty_ident {
            #impls
        }
    }
}

fn gen_child_standard_item(
    child: &StandardDef,
    parent_ident: &Ident,
    is_ordered: bool,
) -> TokenStream {
    let ty_ident = &child.name;
    let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
    let parent_data_ident = Ident::new(&format!("{}Data", parent_ident), parent_ident.span());
    let manager_ident = method_ident_for("manage", &child.name);

    let (basic_methods, basic_impls) = (
        quote! {
            async fn get(ctx: __ctx!(), id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
            async fn update(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                async fn unchecked_list(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
        },
        quote! {
            async fn get(ctx: __ctx!(), id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                ctx.$ctx_repo_accessor().await?.#manager_ident().get(id).await
            }
            async fn update(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                ctx.$ctx_repo_accessor().await?.#manager_ident().update(self).await
            }
                async fn unchecked_list(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                    let tmp_dummy = #parent_ident {
                        id: parent_id,
                        data: #parent_data_ident::default(),
                        auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                    };
                    ctx.$ctx_repo_accessor().await?.#manager_ident().query_all(&tmp_dummy).await
                }
        },
    );

    let (add_methods, add_impls) = if is_ordered {
        (
            quote! {
                async fn unchecked_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                async fn unchecked_batch_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
            },
            quote! {
                async fn unchecked_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                    let tmp_dummy = #parent_ident {
                        id: parent_id,
                        data: #parent_data_ident::default(),
                        auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                    };
                    ctx.$ctx_repo_accessor().await?.#manager_ident().add(&tmp_dummy, data, after).await
                }
                async fn unchecked_batch_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                    let tmp_dummy = #parent_ident {
                        id: parent_id,
                        data: #parent_data_ident::default(),
                        auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                    };
                    ctx.$ctx_repo_accessor().await?.#manager_ident().batch_add(&tmp_dummy, data, after).await
                }
            },
        )
    } else {
        (
            quote! {
                async fn unchecked_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                async fn unchecked_batch_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
            },
            quote! {
                async fn unchecked_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                    let tmp_dummy = #parent_ident {
                        id: parent_id,
                        data: #parent_data_ident::default(),
                        auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                    };
                    ctx.$ctx_repo_accessor().await?.#manager_ident().add(&tmp_dummy, data).await
                }
                async fn unchecked_batch_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                    let tmp_dummy = #parent_ident {
                        id: parent_id,
                        data: #parent_data_ident::default(),
                        auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                    };
                    ctx.$ctx_repo_accessor().await?.#manager_ident().batch_add(&tmp_dummy, data).await
                }
            },
        )
    };

    let (delete_methods, delete_impls) = if child.has_children() {
        (
            quote! {
                async fn delete_recursive(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                #[allow(non_snake_case)]
                async fn delete_non_recursive_DANGEROUS(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                #[allow(non_snake_case)]
                async fn batch_delete_non_recursive_DANGEROUS(ctx: __ctx!(), items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError>;
            },
            quote! {
                async fn delete_recursive(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().delete_recursive(self).await
                }
                #[allow(non_snake_case)]
                async fn delete_non_recursive_DANGEROUS(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().delete_non_recursive(self).await
                }
                #[allow(non_snake_case)]
                async fn batch_delete_non_recursive_DANGEROUS(ctx: __ctx!(), items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_non_recursive(items).await
                }
            },
        )
    } else {
        (
            quote! {
                async fn delete(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                async fn batch_delete(ctx: __ctx!(), items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError>;
            },
            quote! {
                async fn delete(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().delete(self).await
                }
                async fn batch_delete(ctx: __ctx!(), items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError> {
                    ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete(items).await
                }
            },
        )
    };

    let (ordered_grandchild_methods, ordered_grandchild_impls) = child
        .ordered_children
        .iter()
        .map(|grandchild| {
            let gc_ident = grandchild;
            let gc_data_ident = Ident::new(&format!("{}Data", gc_ident), gc_ident.span());
            let gc_manager_ident = method_ident_for("manage", gc_ident);
            let base_pascal = stripped_pascal(ty_ident, gc_ident);
            let singular_snake = to_snake_case(&base_pascal);
            let plural_snake = to_snake_case(&pluralize_pascal(&base_pascal));
            let add_fn = Ident::new(&format!("add_{}", singular_snake), gc_ident.span());
            let batch_add_fn = Ident::new(&format!("batch_add_{}", plural_snake), gc_ident.span());
            let list_fn = Ident::new(&format!("list_{}", plural_snake), gc_ident.span());
            (
                quote! {
                    async fn #add_fn(&self, ctx: __ctx!(), data: #gc_data_ident, after: ::std::option::Option<& #gc_ident>) -> ::std::result::Result<#gc_ident, ::fractic_server_error::ServerError>;
                    async fn #batch_add_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#gc_data_ident>, after: ::std::option::Option<& #gc_ident>) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError>;
                    async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #add_fn(&self, ctx: __ctx!(), data: #gc_data_ident, after: ::std::option::Option<& #gc_ident>) -> ::std::result::Result<#gc_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#gc_manager_ident().add(self, data, after).await
                    }
                    async fn #batch_add_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#gc_data_ident>, after: ::std::option::Option<& #gc_ident>) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#gc_manager_ident().batch_add(self, data, after).await
                    }
                    async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#gc_manager_ident().query_all(self).await
                    }
                }
            )
        })
        .unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    let (unordered_grandchild_methods, unordered_grandchild_impls) = child
        .unordered_children
        .iter()
        .map(|grandchild| {
            let gc_ident = grandchild;
            let gc_data_ident = Ident::new(&format!("{}Data", gc_ident), gc_ident.span());
            let gc_manager_ident = method_ident_for("manage", gc_ident);
            let base_pascal = stripped_pascal(ty_ident, gc_ident);
            let singular_snake = to_snake_case(&base_pascal);
            let plural_snake = to_snake_case(&pluralize_pascal(&base_pascal));
            let add_fn = Ident::new(&format!("add_{}", singular_snake), gc_ident.span());
            let batch_add_fn = Ident::new(&format!("batch_add_{}", plural_snake), gc_ident.span());
            let list_fn = Ident::new(&format!("list_{}", plural_snake), gc_ident.span());
            (
                quote! {
                    async fn #add_fn(&self, ctx: __ctx!(), data: #gc_data_ident) -> ::std::result::Result<#gc_ident, ::fractic_server_error::ServerError>;
                    async fn #batch_add_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#gc_data_ident>) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError>;
                    async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #add_fn(&self, ctx: __ctx!(), data: #gc_data_ident) -> ::std::result::Result<#gc_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#gc_manager_ident().add(self, data).await
                    }
                    async fn #batch_add_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#gc_data_ident>) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#gc_manager_ident().batch_add(self, data).await
                    }
                    async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#gc_manager_ident().query_all(self).await
                    }
                }
            )
        })
        .unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    let (batch_methods, batch_impls) = child
        .batch_children
        .iter()
        .map(|batch| {
            let b_ident = batch;
            let b_data_ident = Ident::new(&format!("{}Data", b_ident), b_ident.span());
            let b_manager_ident = method_ident_for("manage", b_ident);
            let base_pascal = stripped_pascal(ty_ident, b_ident);
            let plural_snake = to_snake_case(&pluralize_pascal(&base_pascal));
            let list_fn = Ident::new(&format!("list_{}", plural_snake), b_ident.span());
            let del_all_fn = Ident::new(&format!("batch_delete_all_{}", plural_snake), b_ident.span());
            let replace_all_fn = Ident::new(&format!("batch_replace_all_{}", plural_snake), b_ident.span());
            (
                quote! {
                    async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#b_ident>, ::fractic_server_error::ServerError>;
                    async fn #del_all_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                    async fn #replace_all_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#b_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#b_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#b_manager_ident().query_all(self).await
                    }
                    async fn #del_all_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#b_manager_ident().batch_delete_all(self).await
                    }
                    async fn #replace_all_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#b_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#b_manager_ident().batch_replace_all_ordered(self, data).await
                    }
                }
            )
        })
        .unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    let (singleton_child_methods, singleton_child_impls) = child
        .singleton_children
        .iter()
        .map(|singleton_child| {
            let s_ident = singleton_child;
            let s_data_ident = Ident::new(&format!("{}Data", s_ident), s_ident.span());
            let s_manager_ident = method_ident_for("manage", s_ident);
            let base_pascal = stripped_pascal(ty_ident, s_ident);
            let singular_snake = to_snake_case(&base_pascal);
            let get_fn = Ident::new(&format!("get_{}", singular_snake), s_ident.span());
            let set_fn = Ident::new(&format!("set_{}", singular_snake), s_ident.span());
            let delete_fn = Ident::new(&format!("delete_{}", singular_snake), s_ident.span());
            (
                quote! {
                    async fn #get_fn(&self, ctx: __ctx!()) -> ::std::result::Result<#s_ident, ::fractic_server_error::ServerError>;
                    async fn #set_fn(&self, ctx: __ctx!(), data: #s_data_ident) -> ::std::result::Result<#s_ident, ::fractic_server_error::ServerError>;
                    async fn #delete_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #get_fn(&self, ctx: __ctx!()) -> ::std::result::Result<#s_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#s_manager_ident().get(self).await
                    }
                    async fn #set_fn(&self, ctx: __ctx!(), data: #s_data_ident) -> ::std::result::Result<#s_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#s_manager_ident().set(self, data).await
                    }
                    async fn #delete_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#s_manager_ident().delete(self).await
                    }
                }
            )
        })
        .unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    let (singleton_family_child_methods, singleton_family_child_impls) = child
        .singleton_family_children
        .iter()
        .map(|singleton_family_child| {
            let s_ident = singleton_family_child;
            let s_data_ident = Ident::new(&format!("{}Data", s_ident), s_ident.span());
            let s_manager_ident = method_ident_for("manage", s_ident);
            let base_pascal = stripped_pascal(ty_ident, s_ident);
            let singular_snake = to_snake_case(&base_pascal);
            let plural_snake = to_snake_case(&pluralize_pascal(&base_pascal));

            let get_fn = Ident::new(&format!("get_{}", singular_snake), s_ident.span());
            let set_fn = Ident::new(&format!("set_{}", singular_snake), s_ident.span());
            let batch_set_fn = Ident::new(&format!("batch_set_{}", plural_snake), s_ident.span());
            let delete_fn = Ident::new(&format!("delete_{}", singular_snake), s_ident.span());
            let batch_delete_fn = Ident::new(&format!("batch_delete_{}", plural_snake), s_ident.span());
            let list_fn = Ident::new(&format!("list_{}", plural_snake), s_ident.span());
            let batch_delete_all_fn = Ident::new(&format!("batch_delete_all_{}", plural_snake), s_ident.span());

            (
                quote! {
                    async fn #get_fn(&self, ctx: __ctx!(), key: &str) -> ::std::result::Result<#s_ident, ::fractic_server_error::ServerError>;
                    async fn #set_fn(&self, ctx: __ctx!(), data: #s_data_ident) -> ::std::result::Result<#s_ident, ::fractic_server_error::ServerError>;
                    async fn #batch_set_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#s_data_ident>) -> ::std::result::Result<::std::vec::Vec<#s_ident>, ::fractic_server_error::ServerError>;
                    async fn #delete_fn(&self, ctx: __ctx!(), key: &str) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                    async fn #batch_delete_fn(&self, ctx: __ctx!(), keys: ::std::vec::Vec<&str>) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                    async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#s_ident>, ::fractic_server_error::ServerError>;
                    async fn #batch_delete_all_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #get_fn(&self, ctx: __ctx!(), key: &str) -> ::std::result::Result<#s_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#s_manager_ident().get(self, key).await
                    }
                    async fn #set_fn(&self, ctx: __ctx!(), data: #s_data_ident) -> ::std::result::Result<#s_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#s_manager_ident().set(self, data).await
                    }
                    async fn #batch_set_fn(&self, ctx: __ctx!(), data: ::std::vec::Vec<#s_data_ident>) -> ::std::result::Result<::std::vec::Vec<#s_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#s_manager_ident().batch_set(self, data).await
                    }
                    async fn #delete_fn(&self, ctx: __ctx!(), key: &str) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#s_manager_ident().delete(self, key).await
                    }
                    async fn #batch_delete_fn(&self, ctx: __ctx!(), keys: ::std::vec::Vec<&str>) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#s_manager_ident().batch_delete(self, keys).await
                    }
                    async fn #list_fn(&self, ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#s_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#s_manager_ident().query_all(self).await
                    }
                    async fn #batch_delete_all_fn(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#s_manager_ident().batch_delete_all(self).await
                    }
                }
            )
        })
        .unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

    quote! {
        pub trait #trait_ident {
            #basic_methods
            #add_methods
            #delete_methods
            #(#ordered_grandchild_methods)*
            #(#unordered_grandchild_methods)*
            #(#batch_methods)*
            #(#singleton_child_methods)*
            #(#singleton_family_child_methods)*
        }
        impl #trait_ident for #ty_ident {
            #basic_impls
            #add_impls
            #delete_impls
            #(#ordered_grandchild_impls)*
            #(#unordered_grandchild_impls)*
            #(#batch_impls)*
            #(#singleton_child_impls)*
            #(#singleton_family_child_impls)*
        }
    }
}

fn gen_child_singleton_item(singleton: &SingletonDef, parent_ident: &Ident) -> TokenStream {
    let ty_ident = &singleton.name;
    let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
    let parent_data_ident = Ident::new(&format!("{}Data", parent_ident), parent_ident.span());
    let manager_ident = method_ident_for("manage", ty_ident);
    let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

    let methods = quote! {
        async fn unchecked_get(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
        async fn unchecked_set(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
        async fn unchecked_delete(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
    };
    let impls = quote! {
        async fn unchecked_get(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().get(&tmp_dummy).await
        }
        async fn unchecked_set(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().set(&tmp_dummy, data).await
        }
        async fn unchecked_delete(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().delete(&tmp_dummy).await
        }
    };

    quote! {
        pub trait #trait_ident {
            #methods
        }
        impl #trait_ident for #ty_ident {
            #impls
        }
    }
}

fn gen_child_singleton_family_item(
    singleton_family: &SingletonFamilyDef,
    parent_ident: &Ident,
) -> TokenStream {
    let ty_ident = &singleton_family.name;
    let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
    let parent_data_ident = Ident::new(&format!("{}Data", parent_ident), parent_ident.span());
    let manager_ident = method_ident_for("manage", ty_ident);
    let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

    let methods = quote! {
        async fn unchecked_get(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, key: &str) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
        async fn unchecked_set(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
        async fn unchecked_batch_set(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
        async fn unchecked_delete(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, key: &str) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
        async fn unchecked_batch_delete(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, keys: ::std::vec::Vec<&str>) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
        async fn unchecked_list(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
        async fn unchecked_batch_delete_all(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
    };
    let impls = quote! {
        async fn unchecked_get(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, key: &str) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().get(&tmp_dummy, key).await
        }
        async fn unchecked_set(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().set(&tmp_dummy, data).await
        }
        async fn unchecked_batch_set(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().batch_set(&tmp_dummy, data).await
        }
        async fn unchecked_delete(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, key: &str) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().delete(&tmp_dummy, key).await
        }
        async fn unchecked_batch_delete(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, keys: ::std::vec::Vec<&str>) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete(&tmp_dummy, keys).await
        }
        async fn unchecked_list(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().query_all(&tmp_dummy).await
        }
        async fn unchecked_batch_delete_all(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
            let tmp_dummy = #parent_ident {
                id: parent_id,
                data: #parent_data_ident::default(),
                auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
            };
            ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_all(&tmp_dummy).await
        }
    };

    quote! {
        pub trait #trait_ident {
            #methods
        }
        impl #trait_ident for #ty_ident {
            #impls
        }
    }
}

fn method_ident_for(prefix: &str, ident: &Ident) -> Ident {
    let snake = to_snake_case(&ident.to_string());
    let name = format!("{}_{}", prefix, snake);
    Ident::new(&name, ident.span())
}

fn stripped_pascal(parent: &Ident, child: &Ident) -> String {
    let p = parent.to_string();
    let c = child.to_string();
    if c.starts_with(&p) {
        let rem = &c[p.len()..];
        if rem.is_empty() { c } else { rem.to_string() }
    } else {
        c
    }
}

/// Very small heuristic pluralizer.
fn pluralize_pascal(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    if lower.ends_with('y')
        && !matches!(
            lower
                .as_bytes()
                .get(lower.len().saturating_sub(2))
                .map(|c| *c as char),
            Some('a' | 'e' | 'i' | 'o' | 'u')
        )
    {
        let mut base = s.to_string();
        base.pop();
        base.push_str("ies");
        base
    } else if lower.ends_with('s')
        || lower.ends_with('x')
        || lower.ends_with('z')
        || lower.ends_with("ch")
        || lower.ends_with("sh")
    {
        let mut base = s.to_string();
        base.push_str("es");
        base
    } else {
        let mut base = s.to_string();
        base.push('s');
        base
    }
}
