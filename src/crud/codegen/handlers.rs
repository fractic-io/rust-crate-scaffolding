use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{crud::model::ConfigModel, helpers::to_snake_case};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let repo_name_snake = to_snake_case(&repo_name.to_string());
    let macro_name_ident = Ident::new(
        &format!("generate_{}_handlers", repo_name_snake),
        repo_name.span(),
    );

    // Return type of all CRUD operations. This is made a serializable untagged
    // enum to transparently provide callers with the JSON they expect given the
    // operation requested (without any additional layers of wrapping).
    //
    // If used together with fractic_aws_apigateway's router handling, the Crud
    // and OwnedCrud specs require handlers of type (CrudOperation<T>) ->
    // Result<impl serde::Serialize, ServerError>, so this enum satisfies that
    // requirement.
    let crud_result_enum = quote! {
        #[derive(::serde::Serialize)]
        #[serde(untagged)]
        pub enum __CrudOperationResult<T>
        where
            T: ::fractic_aws_dynamo::schema::DynamoObject + ::serde::Serialize,
        {
            Created { created_id: ::fractic_aws_dynamo::schema::PkSk },
            CreatedBatch { created_ids: ::std::vec::Vec<::fractic_aws_dynamo::schema::PkSk> },
            Read(T),
            Items(::std::vec::Vec<T>),
            Unit(()),
        }
    };

    // Build handlers for root types.
    let root_handlers = model.root_objects.iter().map(|root| {
        let ty_ident = &root.name;
        let manager_ident = method_ident_for("manage", ty_ident);
        let handler_ident = method_ident_for_with_suffix("manage", ty_ident, "_handler");
        let has_children = root.has_children();

        let delete_arm = if has_children {
            quote! {
                Delete { id, non_recursive } => {
                    let __item = __repo.#manager_ident().get(id).await?;
                    if non_recursive {
                        __repo.#manager_ident().delete_non_recursive(__item).await?;
                    } else {
                        __repo.#manager_ident().delete_recursive(__item).await?;
                    }
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        } else {
            quote! {
                Delete { id, non_recursive: _ } => {
                    let __item = __repo.#manager_ident().get(id).await?;
                    __repo.#manager_ident().delete(__item).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        };
        let delete_batch_arm = if has_children {
            quote! {
                DeleteBatch { ids, non_recursive } => {
                    if !non_recursive {
                        return ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("batch delete on {} requires non_recursive=true", stringify!(#ty_ident))
                            ).into()
                        );
                    }
                    let __items = {
                        let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                        ::futures::future::try_join_all(__futs).await?
                    };
                    __repo.#manager_ident().batch_delete_non_recursive(__items).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        } else {
            quote! {
                DeleteBatch { ids, non_recursive: _ } => {
                    let __items = {
                        let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                        ::futures::future::try_join_all(__futs).await?
                    };
                    __repo.#manager_ident().batch_delete(__items).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        };
        let delete_all_arm = if has_children {
            quote! {
                DeleteAll { parent_id, non_recursive } => {
                    if parent_id.is_some() {
                        return ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("delete-all operations on {} do not allow a parent ID", stringify!(#ty_ident))
                            ).into()
                        );
                    }
                    if !non_recursive {
                        return ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("delete-all on {} requires non_recursive=true", stringify!(#ty_ident))
                            ).into()
                        );
                    }
                    __repo.#manager_ident().batch_delete_all_non_recursive().await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        } else {
            quote! {
                DeleteAll { parent_id, non_recursive: _ } => {
                    if parent_id.is_some() {
                        return ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("delete-all operations on {} do not allow a parent ID", stringify!(#ty_ident))
                            ).into()
                        );
                    }
                    __repo.#manager_ident().batch_delete_all().await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        };

        quote! {
            pub async fn #handler_ident(
                operation: ::fractic_aws_apigateway::CrudOperation<#ty_ident>
            ) -> ::std::result::Result<__CrudOperationResult<#ty_ident>, ::fractic_server_error::ServerError> {
                use ::fractic_aws_apigateway::CrudOperation::*;
                let __repo: ::std::sync::Arc<dyn #repo_name> = { __repo_init!() };
                match operation {
                    List { parent_id } => {
                        if parent_id.is_some() {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("list operations on {} do not allow a parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        }
                        let __items = __repo.#manager_ident().query_all().await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Items(__items))
                    }
                    Create { parent_id, after, data } => {
                        if parent_id.is_some() {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("create operations on {} do not allow a parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        }
                        if after.is_some() {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("create operations on {} do not allow an `after` parameter", stringify!(#ty_ident))
                                ).into()
                            );
                        }
                        let __created = __repo.#manager_ident().add(data).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Created { created_id: __created.id })
                    }
                    CreateBatch { parent_id, after, data } => {
                        if parent_id.is_some() {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("batch create operations on {} do not allow a parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        }
                        if after.is_some() {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("batch create operations on {} do not allow an `after` parameter", stringify!(#ty_ident))
                                ).into()
                            );
                        }
                        let __created = __repo.#manager_ident().batch_add(data).await?;
                        let __ids = __created.into_iter().map(|x| x.id).collect::<::std::vec::Vec<_>>();
                        ::std::result::Result::Ok(__CrudOperationResult::CreatedBatch { created_ids: __ids })
                    }
                    Read { id } => {
                        let __item = __repo.#manager_ident().get(id).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Read(__item))
                    }
                    ReadBatch { ids } => {
                        let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                        let __items = ::futures::future::try_join_all(__futs).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Items(__items))
                    }
                    Update { item } => {
                        __repo.#manager_ident().update(&item).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                    }
                    #delete_arm
                    #delete_batch_arm
                    #delete_all_arm
                    ReplaceAll { parent_id, data: _ } => {
                        if parent_id.is_some() {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("replace-all is not supported for {}", stringify!(#ty_ident))
                                ).into()
                            );
                        }
                        ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("replace-all is not supported for {}", stringify!(#ty_ident))
                            ).into()
                        )
                    }
                }
            }
        }
    });

    // Build handlers for ordered children (require parent_id, allow `after`).
    let ordered_handlers = model.ordered_objects.iter().map(|child| {
        let ty_ident = &child.name;
        let (parent_ident, parent_data_ident) = {
            // These idents are used only to create dummy objects for repository
            // methods requiring a &T `parent` argument. Since the dummy object
            // is only needed to satisfy the type system, we can use any valid
            // parent type.
            let p = &child.parents[0];
            let d = Ident::new(&format!("{}Data", p), p.span());
            (p, d)
        };
        let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
        let manager_ident = method_ident_for("manage", ty_ident);
        let handler_ident = method_ident_for_with_suffix("manage", ty_ident, "_handler");
        let has_children = child.has_children();

        let delete_arm = if has_children {
            quote! {
                Delete { id, non_recursive } => {
                    let __item = __repo.#manager_ident().get(id).await?;
                    if non_recursive {
                        __repo.#manager_ident().delete_non_recursive(__item).await?;
                    } else {
                        __repo.#manager_ident().delete_recursive(__item).await?;
                    }
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        } else {
            quote! {
                Delete { id, non_recursive: _ } => {
                    let __item = __repo.#manager_ident().get(id).await?;
                    __repo.#manager_ident().delete(__item).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        };
        let delete_batch_arm = if has_children {
            quote! {
                DeleteBatch { ids, non_recursive } => {
                    if !non_recursive {
                        return ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("batch delete on {} requires non_recursive=true", stringify!(#ty_ident))
                            ).into()
                        );
                    }
                    let __items = {
                        let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                        ::futures::future::try_join_all(__futs).await?
                    };
                    __repo.#manager_ident().batch_delete_non_recursive(__items).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        } else {
            quote! {
                DeleteBatch { ids, non_recursive: _ } => {
                    let __items = {
                        let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                        ::futures::future::try_join_all(__futs).await?
                    };
                    __repo.#manager_ident().batch_delete(__items).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        };

        quote! {
            pub async fn #handler_ident(
                operation: ::fractic_aws_apigateway::CrudOperation<#ty_ident>
            ) -> ::std::result::Result<__CrudOperationResult<#ty_ident>, ::fractic_server_error::ServerError> {
                use ::fractic_aws_apigateway::CrudOperation::*;
                let __repo: ::std::sync::Arc<dyn #repo_name> = { __repo_init!() };
                match operation {
                    List { parent_id } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("list operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        let __items = __repo.#manager_ident().query_all(&__tmp_parent).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Items(__items))
                    }
                    Create { parent_id, after, data } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("create operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        // Create dummy `parent` and `after` objects to satisfy
                        // the type-safety of the CRUD repository methods. In
                        // their internal logic, only the `id` field of these
                        // objects are used, so this is hacky but safe.
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        let mut __tmp_after: ::std::option::Option<#ty_ident> = after.map(|id| #ty_ident {
                            id,
                            data: #ty_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        });
                        let __after_ref: ::std::option::Option<& #ty_ident> = __tmp_after.as_ref();
                        let __created = __repo.#manager_ident().add(&__tmp_parent, data, __after_ref).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Created { created_id: __created.id })
                    }
                    CreateBatch { parent_id, after, data } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("batch create operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        let mut __tmp_after: ::std::option::Option<#ty_ident> = after.map(|id| #ty_ident {
                            id,
                            data: #ty_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        });
                        let __after_ref: ::std::option::Option<& #ty_ident> = __tmp_after.as_ref();
                        let __created = __repo.#manager_ident().batch_add(&__tmp_parent, data, __after_ref).await?;
                        let __ids = __created.into_iter().map(|x| x.id).collect::<::std::vec::Vec<_>>();
                        ::std::result::Result::Ok(__CrudOperationResult::CreatedBatch { created_ids: __ids })
                    }
                    Read { id } => {
                        let __item = __repo.#manager_ident().get(id).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Read(__item))
                    }
                    ReadBatch { ids } => {
                        let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                        let __items = ::futures::future::try_join_all(__futs).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Items(__items))
                    }
                    Update { item } => {
                        __repo.#manager_ident().update(&item).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                    }
                    #delete_arm
                    #delete_batch_arm
                    DeleteAll { parent_id, non_recursive: _ } => {
                        let Some(_parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("delete-all operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("delete-all is not supported for {}", stringify!(#ty_ident))
                            ).into()
                        )
                    }
                    ReplaceAll { parent_id, data: _ } => {
                        let Some(_parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("replace-all operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("replace-all is not supported for {}", stringify!(#ty_ident))
                            ).into()
                        )
                    }
                }
            }
        }
    });

    // Build handlers for unordered children (require parent_id, forbid `after`).
    let unordered_handlers = model.unordered_objects.iter().map(|child| {
        let ty_ident = &child.name;
        let (parent_ident, parent_data_ident) = {
            // These idents are used only to create dummy objects for repository
            // methods requiring a &T `parent` argument. Since the dummy object
            // is only needed to satisfy the type system, we can use any valid
            // parent type.
            let p = &child.parents[0];
            let d = Ident::new(&format!("{}Data", p), p.span());
            (p, d)
        };
        let manager_ident = method_ident_for("manage", ty_ident);
        let handler_ident = method_ident_for_with_suffix("manage", ty_ident, "_handler");
        let has_children = child.has_children();

        let delete_arm = if has_children {
            quote! {
                Delete { id, non_recursive } => {
                    let __item = __repo.#manager_ident().get(id).await?;
                    if non_recursive {
                        __repo.#manager_ident().delete_non_recursive(__item).await?;
                    } else {
                        __repo.#manager_ident().delete_recursive(__item).await?;
                    }
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        } else {
            quote! {
                Delete { id, non_recursive: _ } => {
                    let __item = __repo.#manager_ident().get(id).await?;
                    __repo.#manager_ident().delete(__item).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        };
        let delete_batch_arm = if has_children {
            quote! {
                DeleteBatch { ids, non_recursive } => {
                    if !non_recursive {
                        return ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("batch delete on {} requires non_recursive=true", stringify!(#ty_ident))
                            ).into()
                        );
                    }
                    let __items = {
                        let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                        ::futures::future::try_join_all(__futs).await?
                    };
                    __repo.#manager_ident().batch_delete_non_recursive(__items).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        } else {
            quote! {
                DeleteBatch { ids, non_recursive: _ } => {
                    let __items = {
                        let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                        ::futures::future::try_join_all(__futs).await?
                    };
                    __repo.#manager_ident().batch_delete(__items).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                }
            }
        };

        quote! {
            pub async fn #handler_ident(
                operation: ::fractic_aws_apigateway::CrudOperation<#ty_ident>
            ) -> ::std::result::Result<__CrudOperationResult<#ty_ident>, ::fractic_server_error::ServerError> {
                use ::fractic_aws_apigateway::CrudOperation::*;
                let __repo: ::std::sync::Arc<dyn #repo_name> = { __repo_init!() };
                match operation {
                    List { parent_id } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("list operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        // Create a dummy `parent` object to satisfy the
                        // type-safety of the CRUD repository methods. In their
                        // internal logic, only the `id` field of this object is
                        // used, so this is hacky but safe.
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        let __items = __repo.#manager_ident().query_all(&__tmp_parent).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Items(__items))
                    }
                    Create { parent_id, after, data } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("create operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        if after.is_some() {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("create operations on {} do not allow an `after` parameter", stringify!(#ty_ident))
                                ).into()
                            );
                        }
                        // Create a dummy `parent` object to satisfy the
                        // type-safety of the CRUD repository methods. In their
                        // internal logic, only the `id` field of this object is
                        // used, so this is hacky but safe.
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        let __created = __repo.#manager_ident().add(&__tmp_parent, data).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Created { created_id: __created.id })
                    }
                    CreateBatch { parent_id, after, data } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("batch create operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        if after.is_some() {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("batch create operations on {} do not allow an `after` parameter", stringify!(#ty_ident))
                                ).into()
                            );
                        }
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        let __created = __repo.#manager_ident().batch_add(&__tmp_parent, data).await?;
                        let __ids = __created.into_iter().map(|x| x.id).collect::<::std::vec::Vec<_>>();
                        ::std::result::Result::Ok(__CrudOperationResult::CreatedBatch { created_ids: __ids })
                    }
                    Read { id } => {
                        let __item = __repo.#manager_ident().get(id).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Read(__item))
                    }
                    ReadBatch { ids } => {
                        let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                        let __items = ::futures::future::try_join_all(__futs).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Items(__items))
                    }
                    Update { item } => {
                        __repo.#manager_ident().update(&item).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                    }
                    #delete_arm
                    #delete_batch_arm
                    DeleteAll { parent_id, non_recursive: _ } => {
                        let Some(_parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("delete-all operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("delete-all is not supported for {}", stringify!(#ty_ident))
                            ).into()
                        )
                    }
                    ReplaceAll { parent_id, data: _ } => {
                        let Some(_parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("replace-all operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("replace-all is not supported for {}", stringify!(#ty_ident))
                            ).into()
                        )
                    }
                }
            }
        }
    });

    // Build handlers for batch children (support List, DeleteAll, ReplaceAll only).
    let batch_handlers = model.batch_objects.iter().map(|batch| {
        let ty_ident = &batch.name;
        let (parent_ident, parent_data_ident) = {
            let p = &batch.parents[0];
            let d = Ident::new(&format!("{}Data", p), p.span());
            (p, d)
        };
        let manager_ident = method_ident_for("manage", ty_ident);
        let handler_ident = method_ident_for_with_suffix("manage", ty_ident, "_handler");

        quote! {
            pub async fn #handler_ident(
                operation: ::fractic_aws_apigateway::CrudOperation<#ty_ident>
            ) -> ::std::result::Result<__CrudOperationResult<#ty_ident>, ::fractic_server_error::ServerError> {
                use ::fractic_aws_apigateway::CrudOperation::*;
                let __repo: ::std::sync::Arc<dyn #repo_name> = { __repo_init!() };
                match operation {
                    List { parent_id } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("list operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        let __items = __repo.#manager_ident().query_all(&__tmp_parent).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Items(__items))
                    }
                    DeleteAll { parent_id, non_recursive: _ } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("delete-all operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        __repo.#manager_ident().batch_delete_all(&__tmp_parent).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                    }
                    ReplaceAll { parent_id, data } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("replace-all operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        __repo.#manager_ident().batch_replace_all_ordered(&__tmp_parent, data).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                    }
                    Create { .. }
                    | CreateBatch { .. }
                    | Read { .. }
                    | ReadBatch { .. }
                    | Update { .. }
                    | Delete { .. }
                    | DeleteBatch { .. } => {
                        ::std::result::Result::Err(
                            ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                &format!("operation not supported for batch collection {}", stringify!(#ty_ident))
                            ).into()
                        )
                    }
                }
            }
        }
    });

    // Compose generation macro.
    let root_handlers_iter = root_handlers.into_iter();
    let ordered_handlers_iter = ordered_handlers.into_iter();
    let unordered_handlers_iter = unordered_handlers.into_iter();
    let batch_handlers_iter = batch_handlers.into_iter();
    quote! {
        #[allow(unused_macros)]
        #[macro_export]
        macro_rules! #macro_name_ident {
            ($($repo_init:tt)+) => {
                macro_rules! __repo_init { () => { { $($repo_init)+ } } }
                #crud_result_enum
                #(#root_handlers_iter)*
                #(#ordered_handlers_iter)*
                #(#unordered_handlers_iter)*
                #(#batch_handlers_iter)*
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

fn method_ident_for_with_suffix(prefix: &str, ident: &Ident, suffix: &str) -> Ident {
    let snake = to_snake_case(&ident.to_string());
    let name = format!("{}_{}{}", prefix, snake, suffix);
    Ident::new(&name, ident.span())
}
