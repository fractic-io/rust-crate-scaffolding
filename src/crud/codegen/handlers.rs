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

        let list_arm = quote! {
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
            },
        };
        let create_arm = quote! {
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
            },
        };
        let create_batch_arm = quote! {
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
            },
        };
        let read_arm = quote! {
            Read { id } => {
                let __item = __repo.#manager_ident().get(id).await?;
                ::std::result::Result::Ok(__CrudOperationResult::Read(__item))
            },
        };
        let read_batch_arm = quote! {
            ReadBatch { ids } => {
                let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                let __items = ::futures::future::try_join_all(__futs).await?;
                ::std::result::Result::Ok(__CrudOperationResult::Items(__items))
            },
        };
        let update_arm = quote! {
            Update { item } => {
                __repo.#manager_ident().update(&item).await?;
                ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
            },
        };
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
                },
            }
        } else {
            quote! {
                Delete { id, non_recursive: _ } => {
                    let __item = __repo.#manager_ident().get(id).await?;
                    __repo.#manager_ident().delete(__item).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                },
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
                },
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
                },
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
                },
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
                },
            }
        };
        let replace_all_arm = quote! {
            ReplaceAll { .. } => {
                ::std::result::Result::Err(
                    ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                        &format!("replace-all is not supported for {}", stringify!(#ty_ident))
                    ).into()
                )
            },
        };

        quote! {
            pub async fn #handler_ident(
                operation: ::fractic_aws_apigateway::CrudOperation<#ty_ident>
            ) -> ::std::result::Result<__CrudOperationResult<#ty_ident>, ::fractic_server_error::ServerError> {
                use ::fractic_aws_apigateway::CrudOperation::*;
                let __repo: ::std::sync::Arc<dyn #repo_name> = { __repo_init!() };
                match operation {
                    #list_arm
                    #create_arm
                    #create_batch_arm
                    #read_arm
                    #read_batch_arm
                    #update_arm
                    #delete_arm
                    #delete_batch_arm
                    #delete_all_arm
                    #replace_all_arm
                }
            }
        }
    });

    // Build handlers for children.
    let child_handlers = model
        .ordered_objects
        .iter()
        .map(|child| (child, true))
        .chain(model.unordered_objects.iter().map(|child| (child, false)))
        .map(|(child, is_ordered)| {
            let ty_ident = &child.name;
            let (parent_ident, parent_data_ident) = {
                let p = &child.parents[0];
                let d = Ident::new(&format!("{}Data", p), p.span());
                (p, d)
            };
            let ty_data_ident = Ident::new(&format!("{}Data", ty_ident), ty_ident.span());
            let manager_ident = method_ident_for("manage", ty_ident);
            let handler_ident = method_ident_for_with_suffix("manage", ty_ident, "_handler");
            let has_children = child.has_children();

            let list_arm = quote! {
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
                },
            };
            let create_arm = if is_ordered {
                quote! {
                    Create { parent_id, after, data } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("create operations on {} require a valid parent ID", stringify!(#ty_ident))
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
                        let __created = __repo.#manager_ident().add(&__tmp_parent, data, __after_ref).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Created { created_id: __created.id })
                    },
                }
            } else {
                quote! {
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
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        let __created = __repo.#manager_ident().add(&__tmp_parent, data).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Created { created_id: __created.id })
                    },
                }
            };
            let create_batch_arm = if is_ordered {
                quote! {
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
                    },
                }
            } else {
                quote! {
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
                    },
                }
            };
            let read_arm = quote! {
                Read { id } => {
                    let __item = __repo.#manager_ident().get(id).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Read(__item))
                },
            };
            let read_batch_arm = quote! {
                ReadBatch { ids } => {
                    let __futs = ids.into_iter().map(|id| __repo.#manager_ident().get(id));
                    let __items = ::futures::future::try_join_all(__futs).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Items(__items))
                },
            };
            let update_arm = quote! {
                Update { item } => {
                    __repo.#manager_ident().update(&item).await?;
                    ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                },
            };
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
                    },
                }
            } else {
                quote! {
                    Delete { id, non_recursive: _ } => {
                        let __item = __repo.#manager_ident().get(id).await?;
                        __repo.#manager_ident().delete(__item).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                    },
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
                    },
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
                    },
                }
            };
            let delete_all_arm = if has_children {
                quote! {
                    DeleteAll { parent_id, non_recursive } => {
                        let Some(parent_id) = parent_id else {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("delete-all operations on {} require a valid parent ID", stringify!(#ty_ident))
                                ).into()
                            );
                        };
                        if !non_recursive {
                            return ::std::result::Result::Err(
                                ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                                    &format!("delete-all on {} requires non_recursive=true", stringify!(#ty_ident))
                                ).into()
                            );
                        }
                        let __tmp_parent = #parent_ident {
                            id: parent_id,
                            data: #parent_data_ident::default(),
                            auto_fields: ::fractic_aws_dynamo::schema::AutoFields::default(),
                        };
                        __repo.#manager_ident().batch_delete_all_non_recursive(&__tmp_parent).await?;
                        ::std::result::Result::Ok(__CrudOperationResult::Unit(()))
                    },
                }
            } else {
                quote! {
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
                    },
                }
            };
            let replace_all_arm = quote! {
                ReplaceAll { .. } => {
                    ::std::result::Result::Err(
                        ::fractic_aws_apigateway::InvalidCrudRequestParameters::new(
                            &format!("replace-all is not supported for {}", stringify!(#ty_ident))
                        ).into()
                    )
                },
            };

            quote! {
                pub async fn #handler_ident(
                    operation: ::fractic_aws_apigateway::CrudOperation<#ty_ident>
                ) -> ::std::result::Result<__CrudOperationResult<#ty_ident>, ::fractic_server_error::ServerError> {
                    use ::fractic_aws_apigateway::CrudOperation::*;
                    let __repo: ::std::sync::Arc<dyn #repo_name> = { __repo_init!() };
                    match operation {
                        #list_arm
                        #create_arm
                        #create_batch_arm
                        #read_arm
                        #read_batch_arm
                        #update_arm
                        #delete_arm
                        #delete_batch_arm
                        #delete_all_arm
                        #replace_all_arm
                    }
                }
            }
        });

    // Build handlers for batch children.
    let batch_handlers = model.batch_objects.iter().map(|batch| {
        let ty_ident = &batch.name;
        let (parent_ident, parent_data_ident) = {
            let p = &batch.parents[0];
            let d = Ident::new(&format!("{}Data", p), p.span());
            (p, d)
        };
        let manager_ident = method_ident_for("manage", ty_ident);
        let handler_ident = method_ident_for_with_suffix("manage", ty_ident, "_handler");

        let list_arm = quote! {
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
            },
        };
        let delete_all_arm = quote! {
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
            },
        };
        let replace_all_arm = quote! {
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
            },
        };
        let unsupported_arm = quote! {
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
            },
        };

        quote! {
            pub async fn #handler_ident(
                operation: ::fractic_aws_apigateway::CrudOperation<#ty_ident>
            ) -> ::std::result::Result<__CrudOperationResult<#ty_ident>, ::fractic_server_error::ServerError> {
                use ::fractic_aws_apigateway::CrudOperation::*;
                let __repo: ::std::sync::Arc<dyn #repo_name> = { __repo_init!() };
                match operation {
                    #list_arm
                    #delete_all_arm
                    #replace_all_arm
                    #unsupported_arm
                }
            }
        }
    });

    let root_handlers_iter = root_handlers.into_iter();
    let child_handlers_iter = child_handlers.into_iter();
    let batch_handlers_iter = batch_handlers.into_iter();
    quote! {
        #[allow(unused_macros)]
        #[macro_export]
        macro_rules! #macro_name_ident {
            ($($repo_init:tt)+) => {
                macro_rules! __repo_init { () => { { $($repo_init)+ } } }
                #crud_result_enum
                #(#root_handlers_iter)*
                #(#child_handlers_iter)*
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
