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
        let has_children = !root.ordered_children.is_empty()
            || !root.unordered_children.is_empty()
            || !root.batch_children.is_empty();
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
        let has_children = !root.ordered_children.is_empty()
            || !root.unordered_children.is_empty()
            || !root.batch_children.is_empty();
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
        let has_children = !root.ordered_children.is_empty()
            || !root.unordered_children.is_empty()
            || !root.batch_children.is_empty();
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

    // Fields, inits, trait impls for ordered children.
    let ordered_child_fields = model.ordered_children.iter().flat_map(|child| {
        let ty_ident = &child.name;
        let has_children = !child.ordered_children.is_empty()
            || !child.unordered_children.is_empty()
            || !child.batch_children.is_empty();
        if child.parents.len() == 1 {
            let field_ident = method_ident_for("manage", &child.name);
            let p = &child.parents[0];
            Some(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChildImpl<#ty_ident, #p>
            })
            .map(|tokens| {
                if has_children {
                    quote! {
                        #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildrenImpl<#ty_ident, #p>
                    }
                } else {
                    tokens
                }
            })
        } else {
            let fields = child.parents.iter().map(|p| {
                let field_ident = Ident::new(
                    &format!("{}_for_{}", method_ident_for("manage", &child.name), to_snake_case(&p.to_string())),
                    child.name.span(),
                );
                if has_children {
                    quote! {
                        #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildrenImpl<#ty_ident, #p>
                    }
                } else {
                    quote! {
                        #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChildImpl<#ty_ident, #p>
                    }
                }
            });
            Some(quote! { #(#fields,)* })
        }
    });
    let ordered_child_inits = model.ordered_children.iter().flat_map(|child| {
        let has_children = !child.ordered_children.is_empty()
            || !child.unordered_children.is_empty()
            || !child.batch_children.is_empty();
        if child.parents.len() == 1 {
            let field_ident = method_ident_for("manage", &child.name);
            if has_children {
                Some(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildrenImpl::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                })
            } else {
                Some(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChildImpl::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                })
            }
        } else {
            let inits = child.parents.iter().map(|p| {
                let field_ident = Ident::new(
                    &format!("{}_for_{}", method_ident_for("manage", &child.name), to_snake_case(&p.to_string())),
                    child.name.span(),
                );
                if has_children {
                    quote! {
                        #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildrenImpl::new(
                            dynamo_util.clone(),
                            crud_algorithms.clone(),
                        )
                    }
                } else {
                    quote! {
                        #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChildImpl::new(
                            dynamo_util.clone(),
                            crud_algorithms.clone(),
                        )
                    }
                }
            });
            Some(quote! { #(#inits,)* })
        }
    });
    let ordered_child_trait_impls = model.ordered_children.iter().flat_map(|child| {
        let ty_ident = &child.name;
        let has_children = !child.ordered_children.is_empty()
            || !child.unordered_children.is_empty()
            || !child.batch_children.is_empty();
        if child.parents.len() == 1 {
            let method_ident = method_ident_for("manage", &child.name);
            let p = &child.parents[0];
            if has_children {
                Some(quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren<#ty_ident, Parent = #p> {
                        &self.#method_ident
                    }
                })
            } else {
                Some(quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChild<#ty_ident, Parent = #p> {
                        &self.#method_ident
                    }
                })
            }
        } else {
            let methods = child.parents.iter().map(|p| {
                let method_ident = Ident::new(
                    &format!("{}_for_{}", method_ident_for("manage", &child.name), to_snake_case(&p.to_string())),
                    child.name.span(),
                );
                if has_children {
                    quote! {
                        fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren<#ty_ident, Parent = #p> {
                            &self.#method_ident
                        }
                    }
                } else {
                    quote! {
                        fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChild<#ty_ident, Parent = #p> {
                            &self.#method_ident
                        }
                    }
                }
            });
            Some(quote! { #(#methods)* })
        }
    });

    // Fields, inits, trait impls for unordered children.
    let unordered_child_fields = model.unordered_children.iter().flat_map(|child| {
        let ty_ident = &child.name;
        let has_children = !child.ordered_children.is_empty()
            || !child.unordered_children.is_empty()
            || !child.batch_children.is_empty();
        if child.parents.len() == 1 {
            let field_ident = method_ident_for("manage", &child.name);
            let p = &child.parents[0];
            if has_children {
                Some(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildrenImpl<#ty_ident, #p>
                })
            } else {
                Some(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildImpl<#ty_ident, #p>
                })
            }
        } else {
            let fields = child.parents.iter().map(|p| {
                let field_ident = Ident::new(
                    &format!("{}_for_{}", method_ident_for("manage", &child.name), to_snake_case(&p.to_string())),
                    child.name.span(),
                );
                if has_children {
                    quote! {
                        #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildrenImpl<#ty_ident, #p>
                    }
                } else {
                    quote! {
                        #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildImpl<#ty_ident, #p>
                    }
                }
            });
            Some(quote! { #(#fields,)* })
        }
    });
    let unordered_child_inits = model.unordered_children.iter().flat_map(|child| {
        let has_children = !child.ordered_children.is_empty()
            || !child.unordered_children.is_empty()
            || !child.batch_children.is_empty();
        if child.parents.len() == 1 {
            let field_ident = method_ident_for("manage", &child.name);
            if has_children {
                Some(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildrenImpl::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                })
            } else {
                Some(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildImpl::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                })
            }
        } else {
            let inits = child.parents.iter().map(|p| {
                let field_ident = Ident::new(
                    &format!("{}_for_{}", method_ident_for("manage", &child.name), to_snake_case(&p.to_string())),
                    child.name.span(),
                );
                if has_children {
                    quote! {
                        #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildrenImpl::new(
                            dynamo_util.clone(),
                            crud_algorithms.clone(),
                        )
                    }
                } else {
                    quote! {
                        #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildImpl::new(
                            dynamo_util.clone(),
                            crud_algorithms.clone(),
                        )
                    }
                }
            });
            Some(quote! { #(#inits,)* })
        }
    });
    let unordered_child_trait_impls = model.unordered_children.iter().flat_map(|child| {
        let ty_ident = &child.name;
        let has_children = !child.ordered_children.is_empty()
            || !child.unordered_children.is_empty()
            || !child.batch_children.is_empty();
        if child.parents.len() == 1 {
            let method_ident = method_ident_for("manage", &child.name);
            let p = &child.parents[0];
            if has_children {
                Some(quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#ty_ident, Parent = #p> {
                        &self.#method_ident
                    }
                })
            } else {
                Some(quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#ty_ident, Parent = #p> {
                        &self.#method_ident
                    }
                })
            }
        } else {
            let methods = child.parents.iter().map(|p| {
                let method_ident = Ident::new(
                    &format!("{}_for_{}", method_ident_for("manage", &child.name), to_snake_case(&p.to_string())),
                    child.name.span(),
                );
                if has_children {
                    quote! {
                        fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#ty_ident, Parent = #p> {
                            &self.#method_ident
                        }
                    }
                } else {
                    quote! {
                        fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#ty_ident, Parent = #p> {
                            &self.#method_ident
                        }
                    }
                }
            });
            Some(quote! { #(#methods)* })
        }
    });

    // Fields, inits, trait impls for batch children.
    let batch_fields = model.batches.iter().flat_map(|batch| {
        let ty_ident = &batch.name;
        if batch.parents.len() == 1 {
            let field_ident = method_ident_for("manage", &batch.name);
            let p = &batch.parents[0];
            Some(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageBatchChildImpl<#ty_ident, #p>
            })
        } else {
            let fields = batch.parents.iter().map(|p| {
                let field_ident = Ident::new(
                    &format!("{}_for_{}", method_ident_for("manage", &batch.name), to_snake_case(&p.to_string())),
                    batch.name.span(),
                );
                quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageBatchChildImpl<#ty_ident, #p>
                }
            });
            Some(quote! { #(#fields,)* })
        }
    });
    let batch_inits = model.batches.iter().flat_map(|batch| {
        if batch.parents.len() == 1 {
            let field_ident = method_ident_for("manage", &batch.name);
            Some(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageBatchChildImpl::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            })
        } else {
            let inits = batch.parents.iter().map(|p| {
                let field_ident = Ident::new(
                    &format!(
                        "{}_for_{}",
                        method_ident_for("manage", &batch.name),
                        to_snake_case(&p.to_string())
                    ),
                    batch.name.span(),
                );
                quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageBatchChildImpl::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                }
            });
            Some(quote! { #(#inits,)* })
        }
    });
    let batch_trait_impls = model.batches.iter().flat_map(|batch| {
        let ty_ident = &batch.name;
        if batch.parents.len() == 1 {
            let method_ident = method_ident_for("manage", &batch.name);
            let p = &batch.parents[0];
            Some(quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#ty_ident, Parent = #p> {
                    &self.#method_ident
                }
            })
        } else {
            let methods = batch.parents.iter().map(|p| {
                let method_ident = Ident::new(
                    &format!("{}_for_{}", method_ident_for("manage", &batch.name), to_snake_case(&p.to_string())),
                    batch.name.span(),
                );
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#ty_ident, Parent = #p> {
                        &self.#method_ident
                    }
                }
            });
            Some(quote! { #(#methods)* })
        }
    });

    let out = quote! {
        pub struct #impl_struct_ident {
            #(#root_fields,)*
            #(#ordered_child_fields,)*
            #(#unordered_child_fields,)*
            #(#batch_fields,)*
        }

        impl #impl_struct_ident {
            pub async fn new(ctx: __ctx!()) -> ::std::result::Result<Self, ::fractic_server_error::ServerError> {
                let dynamo_util = ::std::sync::Arc::new(::fractic_aws_dynamo::util::DynamoUtil::new(ctx, ctx.$ctx_db_method()).await?);
                let crud_algorithms = ::std::sync::Arc::new(<$crud_algorithms>::new(dynamo_util.clone()));
                Ok(Self {
                    #(#root_inits,)*
                    #(#ordered_child_inits,)*
                    #(#unordered_child_inits,)*
                    #(#batch_inits,)*
                })
            }
        }

        impl #repo_name for #impl_struct_ident {
            #(#root_trait_impls)*
            #(#ordered_child_trait_impls)*
            #(#unordered_child_trait_impls)*
            #(#batch_trait_impls)*
        }
    };
    let out_clone = out.clone();

    quote! {
        #[allow(unused_macros)]
        macro_rules! #macro_name_ident {
            (dyn $ctx_view:path => $ctx_db_method:ident, $crud_algorithms:ty) => {
                macro_rules! __ctx { () => { &dyn $ctx_view } }
                #out
            };
            ($ctx_view:path => $ctx_db_method:ident, $crud_algorithms:ty) => {
                macro_rules! __ctx { () => { & $ctx } }
                #out_clone
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
