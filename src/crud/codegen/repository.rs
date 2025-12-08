use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{crud::model::ConfigModel, helpers::to_snake_case};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;

    let mut ordered_parent_of_impls = Vec::new();
    let mut ordered_manage_methods = Vec::new();
    for ordered in &model.ordered_objects {
        // Parent-child relationships.
        let type_ident = &ordered.name;
        if let Some(parents) = &ordered.parents {
            let parent_impls = parents.iter().map(|parent_ident| {
                quote! {
                    impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
                }
            });
            ordered_parent_of_impls.push(quote! { #(#parent_impls)* });
        }

        // Manage method.
        let method_ident = method_ident_for("manage", &ordered.name);
        let manage_ty = if ordered.parents.is_none() {
            root_manage_ty(ObjectType::Ordered, ordered.has_children(), type_ident)
        } else {
            child_manage_ty(ObjectType::Ordered, ordered.has_children(), type_ident)
        };
        ordered_manage_methods.push(quote! {
            fn #method_ident(&self) -> & #manage_ty;
        });
    }

    let mut unordered_parent_of_impls = Vec::new();
    let mut unordered_manage_methods = Vec::new();
    for unordered in &model.unordered_objects {
        // Parent-child relationships.
        let type_ident = &unordered.name;
        if let Some(parents) = &unordered.parents {
            let parent_impls = parents.iter().map(|parent_ident| {
                quote! {
                    impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
                }
            });
            unordered_parent_of_impls.push(quote! { #(#parent_impls)* });
        }

        // Manage method.
        let method_ident = method_ident_for("manage", &unordered.name);
        let manage_ty = if unordered.parents.is_none() {
            root_manage_ty(ObjectType::Unordered, unordered.has_children(), type_ident)
        } else {
            child_manage_ty(ObjectType::Unordered, unordered.has_children(), type_ident)
        };
        unordered_manage_methods.push(quote! {
            fn #method_ident(&self) -> & #manage_ty;
        });
    }

    let mut batch_parent_of_impls = Vec::new();
    let mut batch_manage_methods = Vec::new();
    for batch in &model.batch_objects {
        // Parent-child relationships.
        let type_ident = &batch.name;
        if let Some(parents) = &batch.parents {
            let parent_impls = parents.iter().map(|parent_ident| {
                quote! {
                    impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
                }
            });
            batch_parent_of_impls.push(quote! { #(#parent_impls)* });
        }

        // Manage method.
        let method_ident = method_ident_for("manage", &batch.name);
        let manage_ty = if batch.parents.is_none() {
            root_manage_ty(ObjectType::Batch, false, type_ident)
        } else {
            child_manage_ty(ObjectType::Batch, false, type_ident)
        };
        batch_manage_methods.push(quote! {
            fn #method_ident(&self) -> & #manage_ty;
        });
    }

    let mut singleton_parent_of_impls = Vec::new();
    let mut singleton_manage_methods = Vec::new();
    for singleton in &model.singleton_objects {
        // Parent-child relationships.
        let type_ident = &singleton.name;
        if let Some(parents) = &singleton.parents {
            let parent_impls = parents.iter().map(|parent_ident| {
                quote! {
                    impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
                }
            });
            singleton_parent_of_impls.push(quote! { #(#parent_impls)* });
        }

        // Manage method.
        let method_ident = method_ident_for("manage", &singleton.name);
        let manage_ty = if singleton.parents.is_none() {
            root_manage_ty(ObjectType::Singleton, false, type_ident)
        } else {
            child_manage_ty(ObjectType::Singleton, false, type_ident)
        };
        singleton_manage_methods.push(quote! {
            fn #method_ident(&self) -> & #manage_ty;
        });
    }

    let mut singleton_family_parent_of_impls = Vec::new();
    let mut singleton_family_manage_methods = Vec::new();
    for singleton_family in &model.singleton_family_objects {
        // Parent-child relationships.
        let type_ident = &singleton_family.name;
        if let Some(parents) = &singleton_family.parents {
            let parent_impls = parents.iter().map(|parent_ident| {
                quote! {
                    impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
                }
            });
            singleton_family_parent_of_impls.push(quote! { #(#parent_impls)* });
        }

        // Manage method.
        let method_ident = method_ident_for("manage", &singleton_family.name);
        let manage_ty = if singleton_family.parents.is_none() {
            root_manage_ty(ObjectType::SingletonFamily, false, type_ident)
        } else {
            child_manage_ty(ObjectType::SingletonFamily, false, type_ident)
        };
        singleton_family_manage_methods.push(quote! {
            fn #method_ident(&self) -> & #manage_ty;
        });
    }

    quote! {
        #(#ordered_parent_of_impls)*
        #(#unordered_parent_of_impls)*
        #(#batch_parent_of_impls)*
        #(#singleton_parent_of_impls)*
        #(#singleton_family_parent_of_impls)*

        pub trait #repo_name: ::std::marker::Send + ::std::marker::Sync {
            #(#ordered_manage_methods)*
            #(#unordered_manage_methods)*
            #(#batch_manage_methods)*
            #(#singleton_manage_methods)*
            #(#singleton_family_manage_methods)*
        }
    }
}

fn method_ident_for(prefix: &str, ident: &Ident) -> Ident {
    let snake = to_snake_case(&ident.to_string());
    let name = format!("{}_{}", prefix, snake);
    Ident::new(&name, ident.span())
}

#[derive(Copy, Clone)]
pub(crate) enum ObjectType {
    Ordered,
    Unordered,
    Batch,
    Singleton,
    SingletonFamily,
}

pub(crate) fn root_manage_ty(
    kind: ObjectType,
    has_children: bool,
    ty_ident: &Ident,
) -> TokenStream {
    match (kind, has_children) {
        (ObjectType::Ordered, false) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootOrdered<#ty_ident> }
        }
        (ObjectType::Ordered, true) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootOrderedWithChildren<#ty_ident> }
        }
        (ObjectType::Unordered, false) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootUnordered<#ty_ident> }
        }
        (ObjectType::Unordered, true) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootUnorderedWithChildren<#ty_ident> }
        }
        (ObjectType::Batch, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootBatch<#ty_ident> }
        }
        (ObjectType::Singleton, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootSingleton<#ty_ident> }
        }
        (ObjectType::SingletonFamily, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootSingletonFamily<#ty_ident> }
        }
    }
}

pub(crate) fn child_manage_ty(
    kind: ObjectType,
    has_children: bool,
    ty_ident: &Ident,
) -> TokenStream {
    match (kind, has_children) {
        (ObjectType::Ordered, false) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildOrdered<#ty_ident> }
        }
        (ObjectType::Ordered, true) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildOrderedWithChildren<#ty_ident> }
        }
        (ObjectType::Unordered, false) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildUnordered<#ty_ident> }
        }
        (ObjectType::Unordered, true) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildUnorderedWithChildren<#ty_ident> }
        }
        (ObjectType::Batch, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildBatch<#ty_ident> }
        }
        (ObjectType::Singleton, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildSingleton<#ty_ident> }
        }
        (ObjectType::SingletonFamily, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildSingletonFamily<#ty_ident> }
        }
    }
}
