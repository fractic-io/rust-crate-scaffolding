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
    let root_items: ::std::vec::Vec<TokenStream> = model.root_objects.iter().map(|root| {
        let ty_ident = &root.name;
        let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
        let manager_ident = method_ident_for("manage", &root.name);

        let (basic_methods, basic_impls) = {
            (
                quote! {
                    async fn get(ctx: __ctx!(), id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                    async fn add(ctx: __ctx!(), data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                    async fn batch_add(ctx: __ctx!(), data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
                    async fn update(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn get(ctx: __ctx!(), id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().get(id).await
                    }
                    async fn add(ctx: __ctx!(), data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().add(data).await
                    }
                    async fn batch_add(ctx: __ctx!(), data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_add(data).await
                    }
                    async fn update(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().update(self).await
                    }
                }
            )
        };

        // `delete` methods based on whether the type has children.
        let (delete_methods, delete_impls) = if root.has_children() {
            (
                quote! {
                    async fn delete_recursive(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                    #[allow(non_snake_case)]
                    async fn delete_non_recursive_DANGEROUS(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError>;
                    #[allow(non_snake_case)]
                    async fn batch_delete_non_recursive_DANGEROUS(ctx: __ctx!(), items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError>;
                    async fn list(ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
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
                    async fn list(ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().query_all().await
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
                    async fn list(ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
                    async fn batch_delete_all(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn delete(self, ctx: __ctx!()) -> ::std::result::Result<#ty_data_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().delete(self).await
                    }
                    async fn batch_delete(ctx: __ctx!(), items: ::std::vec::Vec<#ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_data_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete(items).await
                    }
                    async fn list(ctx: __ctx!()) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().query_all().await
                    }
                    async fn batch_delete_all(ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().batch_delete_all().await
                    }
                },
            )
        };

        // Ordered child methods for this root.
        let (ordered_child_methods, ordered_child_impls) = root.ordered_children.iter().map(|child_name| {
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
                }
            )
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

        // Unordered child methods for this root.
        let (unordered_child_methods, unordered_child_impls) = root.unordered_children.iter().map(|child_name| {
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
                }
            )
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

        let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

        quote! {
            pub trait #trait_ident {
                #basic_methods
                #delete_methods
                #(#ordered_child_methods)*
                #(#unordered_child_methods)*
                #(#batch_methods)*
            }
            impl #trait_ident for #ty_ident {
                #basic_impls
                #delete_impls
                #(#ordered_child_impls)*
                #(#unordered_child_impls)*
                #(#batch_impls)*
            }
        }
    }).collect();

    // Build trait + impl blocks for ordered child objects.
    let child_items: ::std::vec::Vec<TokenStream> = model.ordered_objects.iter().map(|c| (true, c)).chain(
        model.unordered_objects.iter().map(|c| (false, c))
    ).map(|(is_ordered, child)| {
        let ty_ident = &child.name;
        let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
        let (parent_ident, parent_data_ident) = {
            // These idents are used only to create dummy objects for unchecked
            // methods. Since the dummy object is only needed to satisfy the
            // type system, we can use any valid parent type.
            let p = &child.parents[0];
            let d = Ident::new(&format!("{}Data", p), p.span());
            (p, d)
        };
        let manager_ident = method_ident_for("manage", &child.name);

        let (basic_methods, basic_impls) = {
            (
                quote! {
                    async fn get(ctx: __ctx!(), id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                    async fn update(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError>;
                },
                quote! {
                    async fn get(ctx: __ctx!(), id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().get(id).await
                    }
                    async fn update(&self, ctx: __ctx!()) -> ::std::result::Result<(), ::fractic_server_error::ServerError> {
                        ctx.$ctx_repo_accessor().await?.#manager_ident().update(self).await
                    }
                }
            )
        };

        // `delete` methods based on whether the child has children of its own.
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

        // Unchecked helper methods that take `parent_id` instead of `&Parent`.
        let (unchecked_methods, unchecked_impls) = if is_ordered {
            (
                quote! {
                    async fn unchecked_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                    async fn unchecked_batch_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>, after: ::std::option::Option<& #ty_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
                    async fn unchecked_list(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
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
                    async fn unchecked_list(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
                        let tmp_dummy = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        ctx.$ctx_repo_accessor().await?.#manager_ident().query_all(&tmp_dummy).await
                    }
                }
            )
        } else {
            (
                quote! {
                    async fn unchecked_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: #ty_data_ident) -> ::std::result::Result<#ty_ident, ::fractic_server_error::ServerError>;
                    async fn unchecked_batch_add(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk, data: ::std::vec::Vec<#ty_data_ident>) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
                    async fn unchecked_list(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError>;
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
                    async fn unchecked_list(ctx: __ctx!(), parent_id: ::fractic_aws_dynamo::schema::PkSk) -> ::std::result::Result<::std::vec::Vec<#ty_ident>, ::fractic_server_error::ServerError> {
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

        // Ordered children of this child.
        let (ordered_grandchild_methods, ordered_grandchild_impls) = child.ordered_children.iter().map(|grandchild| {
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
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

        // Unordered children of this child.
        let (unordered_grandchild_methods, unordered_grandchild_impls) = child.unordered_children.iter().map(|grandchild| {
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
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

        // Batch children of this child.
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
        }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

        let trait_ident = Ident::new(&format!("{}Crud", ty_ident), ty_ident.span());

        quote! {
            pub trait #trait_ident {
                #basic_methods
                #delete_methods
                #unchecked_methods
                #(#ordered_grandchild_methods)*
                #(#unordered_grandchild_methods)*
                #(#batch_methods)*
            }
            impl #trait_ident for #ty_ident {
                #basic_impls
                #delete_impls
                #unchecked_impls
                #(#ordered_grandchild_impls)*
                #(#unordered_grandchild_impls)*
                #(#batch_impls)*
            }
        }
    }).collect();

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
