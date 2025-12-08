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

    // Fields and initializers for ordered collections.
    let mut ordered_root_fields = Vec::new();
    let mut ordered_root_inits = Vec::new();
    let mut ordered_root_trait_impls = Vec::new();
    let mut ordered_child_fields = Vec::new();
    let mut ordered_child_inits = Vec::new();
    let mut ordered_child_trait_impls = Vec::new();
    for ordered in &model.ordered_objects {
        let field_ident = method_ident_for("manage", &ordered.name);
        let method_ident = field_ident.clone();
        let ty_ident = &ordered.name;
        if ordered.is_root() {
            if ordered.has_children() {
                ordered_root_fields.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootOrderedWithChildren<#ty_ident>
                });
                ordered_root_inits.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootOrderedWithChildren::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                });
                ordered_root_trait_impls.push(quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageRootOrderedWithChildren<#ty_ident> {
                        &self.#method_ident
                    }
                });
            } else {
                ordered_root_fields.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootOrdered<#ty_ident>
                });
                ordered_root_inits.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootOrdered::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                });
                ordered_root_trait_impls.push(quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageRootOrdered<#ty_ident> {
                        &self.#method_ident
                    }
                });
            }
        } else if ordered.has_children() {
            ordered_child_fields.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren<#ty_ident>
            });
            ordered_child_inits.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            });
            ordered_child_trait_impls.push(quote! {
                fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren<#ty_ident> {
                    &self.#method_ident
                }
            });
        } else {
            ordered_child_fields.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChild<#ty_ident>
            });
            ordered_child_inits.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageOrderedChild::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            });
            ordered_child_trait_impls.push(quote! {
                fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageOrderedChild<#ty_ident> {
                    &self.#method_ident
                }
            });
        }
    }

    // Fields and initializers for unordered collections.
    let mut unordered_root_fields = Vec::new();
    let mut unordered_root_inits = Vec::new();
    let mut unordered_root_trait_impls = Vec::new();
    let mut unordered_child_fields = Vec::new();
    let mut unordered_child_inits = Vec::new();
    let mut unordered_child_trait_impls = Vec::new();
    for unordered in &model.unordered_objects {
        let field_ident = method_ident_for("manage", &unordered.name);
        let method_ident = field_ident.clone();
        let ty_ident = &unordered.name;
        if unordered.is_root() {
            if unordered.has_children() {
                unordered_root_fields.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootUnorderedWithChildren<#ty_ident>
                });
                unordered_root_inits.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootUnorderedWithChildren::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                });
                unordered_root_trait_impls.push(quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageRootUnorderedWithChildren<#ty_ident> {
                        &self.#method_ident
                    }
                });
            } else {
                unordered_root_fields.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootUnordered<#ty_ident>
                });
                unordered_root_inits.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootUnordered::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                });
                unordered_root_trait_impls.push(quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageRootUnordered<#ty_ident> {
                        &self.#method_ident
                    }
                });
            }
        } else if unordered.has_children() {
            unordered_child_fields.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#ty_ident>
            });
            unordered_child_inits.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            });
            unordered_child_trait_impls.push(quote! {
                fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#ty_ident> {
                    &self.#method_ident
                }
            });
        } else {
            unordered_child_fields.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#ty_ident>
            });
            unordered_child_inits.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            });
            unordered_child_trait_impls.push(quote! {
                fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#ty_ident> {
                    &self.#method_ident
                }
            });
        }
    }

    // Fields, inits, trait impls for batch collections.
    let mut batch_root_fields = Vec::new();
    let mut batch_root_inits = Vec::new();
    let mut batch_root_trait_impls = Vec::new();
    let mut batch_child_fields = Vec::new();
    let mut batch_child_inits = Vec::new();
    let mut batch_child_trait_impls = Vec::new();
    for batch in &model.batch_objects {
        let field_ident = method_ident_for("manage", &batch.name);
        let method_ident = field_ident.clone();
        let ty_ident = &batch.name;
        if batch.is_root() {
            batch_root_fields.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootBatch<#ty_ident>
            });
            batch_root_inits.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootBatch::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            });
            batch_root_trait_impls.push(quote! {
                fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageRootBatch<#ty_ident> {
                    &self.#method_ident
                }
            });
        } else {
            batch_child_fields.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#ty_ident>
            });
            batch_child_inits.push(quote! {
                #field_ident: ::fractic_aws_dynamo::ext::crud::ManageBatchChild::new(
                    dynamo_util.clone(),
                    crud_algorithms.clone(),
                )
            });
            batch_child_trait_impls.push(quote! {
                fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#ty_ident> {
                    &self.#method_ident
                }
            });
        }
    }

    // Fields and initializers for singleton roots/children.
    let mut singleton_root_fields = Vec::new();
    let mut singleton_root_inits = Vec::new();
    let mut singleton_root_trait_impls = Vec::new();
    let mut singleton_child_fields = Vec::new();
    let mut singleton_child_inits = Vec::new();
    let mut singleton_child_trait_impls = Vec::new();
    for singleton in &model.singleton_objects {
        let field_ident = method_ident_for("manage", &singleton.name);
        let method_ident = field_ident.clone();
        let ty_ident = &singleton.name;
        match &singleton.parents {
            Some(_) => {
                singleton_child_fields.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageSingletonChild<#ty_ident>
                });
                singleton_child_inits.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageSingletonChild::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                });
                singleton_child_trait_impls.push(quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageSingletonChild<#ty_ident> {
                        &self.#method_ident
                    }
                });
            }
            None => {
                singleton_root_fields.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootSingleton<#ty_ident>
                });
                singleton_root_inits.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootSingleton::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                });
                singleton_root_trait_impls.push(quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageRootSingleton<#ty_ident> {
                        &self.#method_ident
                    }
                });
            }
        }
    }

    // Fields and initializers for singleton family roots/children.
    let mut singleton_family_root_fields = Vec::new();
    let mut singleton_family_root_inits = Vec::new();
    let mut singleton_family_root_trait_impls = Vec::new();
    let mut singleton_family_child_fields = Vec::new();
    let mut singleton_family_child_inits = Vec::new();
    let mut singleton_family_child_trait_impls = Vec::new();
    for singleton_family in &model.singleton_family_objects {
        let field_ident = method_ident_for("manage", &singleton_family.name);
        let method_ident = field_ident.clone();
        let ty_ident = &singleton_family.name;
        match &singleton_family.parents {
            Some(_) => {
                singleton_family_child_fields.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageSingletonFamilyChild<#ty_ident>
                });
                singleton_family_child_inits.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageSingletonFamilyChild::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                });
                singleton_family_child_trait_impls.push(quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageSingletonFamilyChild<#ty_ident> {
                        &self.#method_ident
                    }
                });
            }
            None => {
                singleton_family_root_fields.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootSingletonFamily<#ty_ident>
                });
                singleton_family_root_inits.push(quote! {
                    #field_ident: ::fractic_aws_dynamo::ext::crud::ManageRootSingletonFamily::new(
                        dynamo_util.clone(),
                        crud_algorithms.clone(),
                    )
                });
                singleton_family_root_trait_impls.push(quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageRootSingletonFamily<#ty_ident> {
                        &self.#method_ident
                    }
                });
            }
        }
    }

    let out = quote! {
        pub struct #impl_struct_ident {
            #(#ordered_root_fields,)*
            #(#unordered_root_fields,)*
            #(#batch_root_fields,)*
            #(#singleton_root_fields,)*
            #(#singleton_family_root_fields,)*
            #(#ordered_child_fields,)*
            #(#unordered_child_fields,)*
            #(#batch_child_fields,)*
            #(#singleton_child_fields,)*
            #(#singleton_family_child_fields,)*
        }

        impl #impl_struct_ident {
            pub async fn new(ctx: __ctx!()) -> ::std::result::Result<Self, ::fractic_server_error::ServerError> {
                let dynamo_util = ::std::sync::Arc::new(::fractic_aws_dynamo::util::DynamoUtil::new(ctx, ctx.$ctx_db_method()).await?);
                let crud_algorithms = ::std::sync::Arc::new(<$crud_algorithms>::new(dynamo_util.clone()));
                Ok(Self {
                    #(#ordered_root_inits,)*
                    #(#unordered_root_inits,)*
                    #(#batch_root_inits,)*
                    #(#singleton_root_inits,)*
                    #(#singleton_family_root_inits,)*
                    #(#ordered_child_inits,)*
                    #(#unordered_child_inits,)*
                    #(#batch_child_inits,)*
                    #(#singleton_child_inits,)*
                    #(#singleton_family_child_inits,)*
                })
            }
        }

        impl #repo_name for #impl_struct_ident {
            #(#ordered_root_trait_impls)*
            #(#unordered_root_trait_impls)*
            #(#batch_root_trait_impls)*
            #(#singleton_root_trait_impls)*
            #(#singleton_family_root_trait_impls)*
            #(#ordered_child_trait_impls)*
            #(#unordered_child_trait_impls)*
            #(#batch_child_trait_impls)*
            #(#singleton_child_trait_impls)*
            #(#singleton_family_child_trait_impls)*
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
            ($ctx:path => $ctx_db_method:ident, $crud_algorithms:ty) => {
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
