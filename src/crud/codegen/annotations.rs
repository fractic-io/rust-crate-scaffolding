use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{crud::model::ConfigModel, helpers::to_snake_case};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let repo_name_snake = to_snake_case(&repo_name.to_string());
    let macro_name_ident = Ident::new(
        &format!("generate_{}_annotations", repo_name_snake),
        repo_name.span(),
    );

    // Build trait + impl blocks for root objects.
    let root_items = model.roots.iter().map(|root| {
        let ty_ident = &root.name;
        let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
        let manager_ident = method_ident_for("manage", &root.name);
        let has_children = !root.children.is_empty() || !root.batch_children.is_empty();

        let (basic_methods, basic_impls) = {
            (
                quote! {
                    async fn get(ctx: &impl $ctx_view, id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                    async fn add(ctx: &impl $ctx_view, data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                    async fn batch_add(ctx: &impl $ctx_view, data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
                    async fn update(&self, ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn get(ctx: &impl $ctx_view, id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().get(id).await
                    }
                    async fn add(ctx: &impl $ctx_view, data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().add(data).await
                    }
                    async fn batch_add(ctx: &impl $ctx_view, data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_add(data).await
                    }
                    async fn update(&self, ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().update(self).await
                    }
                }
            )
        };

        // `delete` methods based on whether the type has children.
        let (delete_methods, delete_impls) = if has_children {
            (
                quote! {
                    async fn delete_recursive(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                    #[allow(non_snake_case)]
                    async fn delete_non_recursive_DANGEROUS(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                    #[allow(non_snake_case)]
                    async fn batch_delete_non_recursive_DANGEROUS(ctx: &impl $ctx_view, items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError>;
                    async fn list(ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
                    #[allow(non_snake_case)]
                    async fn batch_delete_all_non_recursive_DANGEROUS(ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn delete_recursive(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().delete_recursive(self).await
                    }
                    #[allow(non_snake_case)]
                    async fn delete_non_recursive_DANGEROUS(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().delete_non_recursive(self).await
                    }
                    #[allow(non_snake_case)]
                    async fn batch_delete_non_recursive_DANGEROUS(ctx: &impl $ctx_view, items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_non_recursive(items).await
                    }
                    async fn list(ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().query_all().await
                    }
                    #[allow(non_snake_case)]
                    async fn batch_delete_all_non_recursive_DANGEROUS(ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_all_non_recursive().await
                    }
                },
            )
        } else {
            (
                quote! {
                    async fn delete(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                    async fn batch_delete(ctx: &impl $ctx_view, items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError>;
                    async fn list(ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
                    async fn batch_delete_all(ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn delete(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().delete(self).await
                    }
                    async fn batch_delete(ctx: &impl $ctx_view, items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete(items).await
                    }
                    async fn list(ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().query_all().await
                    }
                    async fn batch_delete_all(ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_all().await
                    }
                },
            )
        };

        // Child methods for this root.
        let (child_methods, child_impls) = root.children.iter().map(|child_name| {
            let child_ident = child_name;
            let child_data_ident = Ident::new(&format!("{}Data", child_ident), child_ident.span());
            let child_manager_ident = method_ident_for("manage", child_ident);
            let base_pascal = stripped_pascal(ty_ident, child_ident);
            let child_singular_snake = to_snake_case(&base_pascal);
            let child_plural_snake = to_snake_case(&pluralize_pascal(&base_pascal));
            let add_child_fn = Ident::new(&format!("add_{}", child_singular_snake), child_ident.span());
            let batch_add_children_fn = Ident::new(&format!("batch_add_{}", child_plural_snake), child_ident.span());
            let list_children_fn = Ident::new(&format!("list_{}", child_plural_snake), child_ident.span());
            (
                quote! {
                    async fn #add_child_fn(&self, ctx: &impl $ctx_view, data: #child_data_ident, after: ::std::option::Option<& #child_ident>) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError>;
                    async fn #batch_add_children_fn(&self, ctx: &impl $ctx_view, data: ::std::vec::Vec<#child_data_ident>, after: ::std::option::Option<& #child_ident>) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError>;
                    async fn #list_children_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #add_child_fn(&self, ctx: &impl $ctx_view, data: #child_data_ident, after: ::std::option::Option<& #child_ident>) -> ::std::result::Result<#child_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().add(self, data, after).await
                    }
                    async fn #batch_add_children_fn(&self, ctx: &impl $ctx_view, data: ::std::vec::Vec<#child_data_ident>, after: ::std::option::Option<& #child_ident>) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().batch_add(self, data, after).await
                    }
                    async fn #list_children_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#child_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#child_manager_ident().query_all(self).await
                    }
                }
            )
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

        // Batch-child methods for this root.
        let (batch_methods, batch_impls) = root.batch_children.iter().map(|batch_name| {
            let batch_ident = batch_name;
            let batch_data_ident = Ident::new(&format!("{}Data", batch_ident), batch_ident.span());
            let batch_manager_ident = method_ident_for("manage", batch_ident);
            let base_pascal = stripped_pascal(ty_ident, batch_ident);
            let plural_snake = to_snake_case(&pluralize_pascal(&base_pascal));
            let list_fn = Ident::new(&format!("list_{}", plural_snake), batch_ident.span());
            let del_all_fn = Ident::new(&format!("batch_delete_all_{}", plural_snake), batch_ident.span());
            let replace_all_fn = Ident::new(&format!("batch_replace_all_{}", plural_snake), batch_ident.span());
            (
                quote! {
                    async fn #list_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#batch_ident>, ::fractic_server_error::ServerError>;
                    async fn #del_all_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                    async fn #replace_all_fn(&self, ctx: &impl $ctx_view, data: ::std::vec::Vec<#batch_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #list_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#batch_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#batch_manager_ident().query_all(self).await
                    }
                    async fn #del_all_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#batch_manager_ident().batch_delete_all(self).await
                    }
                    async fn #replace_all_fn(&self, ctx: &impl $ctx_view, data: ::std::vec::Vec<#batch_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#batch_manager_ident().batch_replace_all_ordered(self, data).await
                    }
                }
            )
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

        let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

        quote! {
            pub trait #trait_ident {
                #basic_methods
                #delete_methods
                #(#child_methods)*
                #(#batch_methods)*
            }
            impl #trait_ident for #ty_ident {
                #basic_impls
                #delete_impls
                #(#child_impls)*
                #(#batch_impls)*
            }
        }
    });

    // Build trait + impl blocks for child objects (non-batch).
    let child_items = model.children.iter().map(|child| {
        let ty_ident = &child.name;
        let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
        let parent_ident = &child.parent;
        let parent_data_ident = Ident::new(&format!("{}Data", parent_ident), parent_ident.span());
        let manager_ident = method_ident_for("manage", &child.name);
        let has_children = !child.children.is_empty() || !child.batch_children.is_empty();

        let (basic_methods, basic_impls) = {
            (
                quote! {
                    async fn get(ctx: &impl $ctx_view, id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                    async fn update(&self, ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn get(ctx: &impl $ctx_view, id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().get(id).await
                    }
                    async fn update(&self, ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().update(self).await
                    }
                }
            )
        };

        // `delete` methods based on whether the child has children of its own.
        let (delete_methods, delete_impls) = if has_children {
            (
                quote! {
                    async fn delete_recursive(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                    #[allow(non_snake_case)]
                    async fn delete_non_recursive_DANGEROUS(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                    #[allow(non_snake_case)]
                    async fn batch_delete_non_recursive_DANGEROUS(ctx: &impl $ctx_view, items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn delete_recursive(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().delete_recursive(self).await
                    }
                    #[allow(non_snake_case)]
                    async fn delete_non_recursive_DANGEROUS(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().delete_non_recursive(self).await
                    }
                    #[allow(non_snake_case)]
                    async fn batch_delete_non_recursive_DANGEROUS(ctx: &impl $ctx_view, items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_non_recursive(items).await
                    }
                },
            )
        } else {
            (
                quote! {
                    async fn delete(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                    async fn batch_delete(ctx: &impl $ctx_view, items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn delete(self, ctx: &impl $ctx_view) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().delete(self).await
                    }
                    async fn batch_delete(ctx: &impl $ctx_view, items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete(items).await
                    }
                },
            )
        };

        // Unchecked helper methods that take `parent_id` instead of `&Parent`.
        let (unchecked_methods, unchecked_impls) = {
            let unchecked_add_fn = Ident::new("unchecked_add", ty_ident.span());
            let unchecked_batch_add_fn = Ident::new("unchecked_batch_add", ty_ident.span());
            let unchecked_list_fn = Ident::new("unchecked_list", ty_ident.span());
            (
                quote! {
                    async fn #unchecked_add_fn(ctx: &impl $ctx_view, parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                    async fn #unchecked_batch_add_fn(ctx: &impl $ctx_view, parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
                    async fn #unchecked_list_fn(ctx: &impl $ctx_view, parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #unchecked_add_fn(ctx: &impl $ctx_view, parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                        let tmp_dummy = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        ctx.$ctx_repo_accessor().await?.#manager_ident().add(&tmp_dummy, data, after).await
                    }
                    async fn #unchecked_batch_add_fn(ctx: &impl $ctx_view, parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                        let tmp_dummy = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_add(&tmp_dummy, data, after).await
                    }
                    async fn #unchecked_list_fn(ctx: &impl $ctx_view, parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                        let tmp_dummy = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        ctx.$ctx_repo_accessor().await?.#manager_ident().query_all(&tmp_dummy).await
                    }
                }
            )
        };

        // Children (non-batch) of this child.
        let (child_methods, child_impls) = child.children.iter().map(|grandchild| {
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
                    async fn #add_fn(&self, ctx: &impl $ctx_view, data: #gc_data_ident, after: ::std::option::Option<& #gc_ident>) -> ::std::result::Result<#gc_ident, ::fractic_server_error::ServerError>;
                    async fn #batch_add_fn(&self, ctx: &impl $ctx_view, data: ::std::vec::Vec<#gc_data_ident>, after: ::std::option::Option<& #gc_ident>) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError>;
                    async fn #list_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #add_fn(&self, ctx: &impl $ctx_view, data: #gc_data_ident, after: ::std::option::Option<& #gc_ident>) -> ::std::result::Result<#gc_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#gc_manager_ident().add(self, data, after).await
                    }
                    async fn #batch_add_fn(&self, ctx: &impl $ctx_view, data: ::std::vec::Vec<#gc_data_ident>, after: ::std::option::Option<& #gc_ident>) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#gc_manager_ident().batch_add(self, data, after).await
                    }
                    async fn #list_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#gc_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#gc_manager_ident().query_all(self).await
                    }
                }
            )
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

        // Batch-children of this child.
        let (batch_methods, batch_impls) = child.batch_children.iter().map(|batch| {
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
                    async fn #list_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#b_ident>, ::fractic_server_error::ServerError>;
                    async fn #del_all_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                    async fn #replace_all_fn(&self, ctx: &impl $ctx_view, data: ::std::vec::Vec<#b_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn #list_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<::std::vec::Vec<#b_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#b_manager_ident().query_all(self).await
                    }
                    async fn #del_all_fn(&self, ctx: &impl $ctx_view) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#b_manager_ident().batch_delete_all(self).await
                    }
                    async fn #replace_all_fn(&self, ctx: &impl $ctx_view, data: ::std::vec::Vec<#b_data_ident>) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#b_manager_ident().batch_replace_all_ordered(self, data).await
                    }
                }
            )
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

        let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

        quote! {
            pub trait #trait_ident {
                #basic_methods
                #delete_methods
                #unchecked_methods
                #(#child_methods)*
                #(#batch_methods)*
            }
            impl #trait_ident for #ty_ident {
                #basic_impls
                #delete_impls
                #unchecked_impls
                #(#child_impls)*
                #(#batch_impls)*
            }
        }
    });

    quote! {
        #[allow(unused_macros)]
        #[macro_export]
        macro_rules! #macro_name_ident {
            ($ctx_view:path => $ctx_repo_accessor:ident) => {
                // Roots:
                #(#root_items)*

                // Children:
                #(#child_items)*
            };
        }

        #[allow(unused_imports)]
        pub(crate) use #macro_name_ident;
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
