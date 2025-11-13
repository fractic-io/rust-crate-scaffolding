use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{crud::model::ConfigModel, helpers::to_snake_case};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let repo_name_snake = to_snake_case(&repo_name.to_string());
    let macro_name_ident = Ident::new(
        &format!("generate_{}_impl", repo_name_snake),
        repo_name.span(),
    );
    let impl_struct_ident = Ident::new(&format!("{}Impl", repo_name), repo_name.span());

    // Fields and initializers for roots.
    let root_fields = model.roots.iter().map(|root| {
        let field_ident = method_ident_for("manage", &root.name);
        let ty_ident = &root.name;
        let has_children = !root.children.is_empty() || !root.batch_children.is_empty();
        if has_children {
            quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootWithChildrenImpl<#ty_ident>
            }
        } else {
            quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootImpl<#ty_ident>
            }
        }
    });
    let root_inits = model.roots.iter().map(|root| {
        let field_ident = method_ident_for("manage", &root.name);
        let has_children = !root.children.is_empty() || !root.batch_children.is_empty();
        if has_children {
            quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootWithChildrenImpl::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            }
        } else {
            quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootImpl::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            }
        }
    });
    let root_trait_impls = model.roots.iter().map(|root| {
        let method_ident = method_ident_for("manage", &root.name);
        let ty_ident = &root.name;
        let has_children = !root.children.is_empty() || !root.batch_children.is_empty();
        if has_children {
            quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageRootWithChildren<#ty_ident> {
                    &self.#method_ident
                }
            }
        } else {
            quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageRoot<#ty_ident> {
                    &self.#method_ident
                }
            }
        }
    });

    // Fields, inits, trait impls for children.
    let child_fields = model.children.iter().map(|child| {
        let field_ident = method_ident_for("manage", &child.name);
        let ty_ident = &child.name;
        let parent_ident = &child.parent;
        let has_children = !child.children.is_empty() || !child.batch_children.is_empty();
        if has_children {
            quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageChildWithChildrenImpl<#ty_ident, #parent_ident>
            }
        } else {
            quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageChildImpl<#ty_ident, #parent_ident>
            }
        }
    });
    let child_inits = model.children.iter().map(|child| {
        let field_ident = method_ident_for("manage", &child.name);
        let has_children = !child.children.is_empty() || !child.batch_children.is_empty();
        if has_children {
            quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageChildWithChildrenImpl::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            }
        } else {
            quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageChildImpl::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            }
        }
    });
    let child_trait_impls = model.children.iter().map(|child| {
        let method_ident = method_ident_for("manage", &child.name);
        let ty_ident = &child.name;
        let parent_ident = &child.parent;
        let has_children = !child.children.is_empty() || !child.batch_children.is_empty();
        if has_children {
            quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageChildWithChildren<#ty_ident, Parent = #parent_ident> {
                    &self.#method_ident
                }
            }
        } else {
            quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageChild<#ty_ident, Parent = #parent_ident> {
                    &self.#method_ident
                }
            }
        }
    });

    // Fields, inits, trait impls for batch children.
    let batch_fields = model.batches.iter().map(|batch| {
        let field_ident = method_ident_for("manage", &batch.name);
        let ty_ident = &batch.name;
        let parent_ident = &batch.parent;
        quote! {
            #field_ident: ::fractic_aws_dynamo::ext::crud::ManageBatchChildImpl<#ty_ident, #parent_ident>
        }
    });
    let batch_inits = model.batches.iter().map(|batch| {
        let field_ident = method_ident_for("manage", &batch.name);
        quote! {
            #field_ident: ::fractic_aws_dynamo::ext::crud::ManageBatchChildImpl::new(
                dynamo_util.clone(),
                crud_algorithms.clone(),
            )
        }
    });
    let batch_trait_impls = model.batches.iter().map(|batch| {
        let method_ident = method_ident_for("manage", &batch.name);
        let ty_ident = &batch.name;
        let parent_ident = &batch.parent;
        quote! {
            fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#ty_ident, Parent = #parent_ident> {
                &self.#method_ident
            }
        }
    });

    quote! {
        #[allow(unused_macros)]
        macro_rules! #macro_name_ident {
            ($api_ctx_view:ty, $ctx_db_expr:expr, $crud_algorithms:ty) => {
                pub struct #impl_struct_ident {
                    #(#root_fields,)*
                    #(#child_fields,)*
                    #(#batch_fields,)*
                }

                impl #impl_struct_ident {
                    pub async fn new(ctx: &dyn $api_ctx_view) -> ::std::result::Result<Self, ::fractic_server_error::ServerError> {
                        let dynamo_util = ::std::sync::Arc::new(::fractic_aws_dynamo::util::DynamoUtil::new(ctx, $ctx_db_expr).await?);
                        let crud_algorithms = ::std::sync::Arc::new(<$crud_algorithms>::new(dynamo_util.clone()));
                        Ok(Self {
                            #(#root_inits,)*
                            #(#child_inits,)*
                            #(#batch_inits,)*
                        })
                    }
                }

                impl #repo_name for #impl_struct_ident {
                    #(#root_trait_impls)*
                    #(#child_trait_impls)*
                    #(#batch_trait_impls)*
                }
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
