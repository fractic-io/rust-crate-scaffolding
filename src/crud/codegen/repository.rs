use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{crud::model::ConfigModel, helpers::to_snake_case};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;

    let mut ordered_parent_of_impls = Vec::new();
    let mut ordered_root_manage_methods = Vec::new();
    let mut ordered_child_manage_methods = Vec::new();
    for ordered in &model.ordered_objects {
        let type_ident = &ordered.name;
        let method_ident = method_ident_for("manage", &ordered.name);
        if let Some(parents) = &ordered.parents {
            let parent_impls = parents.iter().map(|parent_ident| {
                quote! {
                    impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
                }
            });
            ordered_parent_of_impls.push(quote! {
                #(#parent_impls)*
            });
            let manage_ty =
                child_manage_ty(CollectionKind::Ordered, ordered.has_children(), type_ident);
            ordered_child_manage_methods.push(quote! {
                fn #method_ident(&self) -> & #manage_ty;
            });
        } else {
            let manage_ty =
                root_manage_ty(CollectionKind::Ordered, ordered.has_children(), type_ident);
            ordered_root_manage_methods.push(quote! {
                fn #method_ident(&self) -> & #manage_ty;
            });
        }
    }

    let mut unordered_parent_of_impls = Vec::new();
    let mut unordered_root_manage_methods = Vec::new();
    let mut unordered_child_manage_methods = Vec::new();
    for unordered in &model.unordered_objects {
        let type_ident = &unordered.name;
        let method_ident = method_ident_for("manage", &unordered.name);
        if let Some(parents) = &unordered.parents {
            let parent_impls = parents.iter().map(|parent_ident| {
                quote! {
                    impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
                }
            });
            unordered_parent_of_impls.push(quote! {
                #(#parent_impls)*
            });
            let manage_ty = child_manage_ty(
                CollectionKind::Unordered,
                unordered.has_children(),
                type_ident,
            );
            unordered_child_manage_methods.push(quote! {
                fn #method_ident(&self) -> & #manage_ty;
            });
        } else {
            let manage_ty = root_manage_ty(
                CollectionKind::Unordered,
                unordered.has_children(),
                type_ident,
            );
            unordered_root_manage_methods.push(quote! {
                fn #method_ident(&self) -> & #manage_ty;
            });
        }
    }

    let mut batch_parent_of_impls = Vec::new();
    let mut batch_root_manage_methods = Vec::new();
    let mut batch_child_manage_methods = Vec::new();
    for batch in &model.batch_objects {
        let type_ident = &batch.name;
        let method_ident = method_ident_for("manage", &batch.name);
        if let Some(parents) = &batch.parents {
            let parent_impls = parents.iter().map(|parent_ident| {
                quote! {
                    impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
                }
            });
            batch_parent_of_impls.push(quote! {
                #(#parent_impls)*
            });
            let manage_ty = child_manage_ty(CollectionKind::Batch, false, type_ident);
            batch_child_manage_methods.push(quote! {
                fn #method_ident(&self) -> & #manage_ty;
            });
        } else {
            let manage_ty = root_manage_ty(CollectionKind::Batch, false, type_ident);
            batch_root_manage_methods.push(quote! {
                fn #method_ident(&self) -> & #manage_ty;
            });
        }
    }

    // Methods for singleton objects.
    let mut singleton_parent_of_impls = Vec::new();
    let mut singleton_root_manage_methods = Vec::new();
    let mut singleton_child_manage_methods = Vec::new();
    for singleton in &model.singleton_objects {
        let type_ident = &singleton.name;
        let method_ident = method_ident_for("manage", type_ident);
        if let Some(parents) = &singleton.parents {
            let parent_impls = parents.iter().map(|parent_ident| {
                quote! {
                    impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
                }
            });
            singleton_parent_of_impls.push(quote! {
                #(#parent_impls)*
            });
            let manage_ty = child_manage_ty(CollectionKind::Singleton, false, type_ident);
            singleton_child_manage_methods.push(quote! {
                fn #method_ident(&self) -> & #manage_ty;
            });
        } else {
            let manage_ty = root_manage_ty(CollectionKind::Singleton, false, type_ident);
            singleton_root_manage_methods.push(quote! {
                fn #method_ident(&self) -> & #manage_ty;
            });
        }
    }

    // Methods for singleton family objects.
    let mut singleton_family_parent_of_impls = Vec::new();
    let mut singleton_family_root_manage_methods = Vec::new();
    let mut singleton_family_child_manage_methods = Vec::new();
    for singleton_family in &model.singleton_family_objects {
        let type_ident = &singleton_family.name;
        let method_ident = method_ident_for("manage", type_ident);
        if let Some(parents) = &singleton_family.parents {
            let parent_impls = parents.iter().map(|parent_ident| {
                quote! {
                    impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
                }
            });
            singleton_family_parent_of_impls.push(quote! {
                #(#parent_impls)*
            });
            let manage_ty = child_manage_ty(CollectionKind::SingletonFamily, false, type_ident);
            singleton_family_child_manage_methods.push(quote! {
                fn #method_ident(&self) -> & #manage_ty;
            });
        } else {
            let manage_ty = root_manage_ty(CollectionKind::SingletonFamily, false, type_ident);
            singleton_family_root_manage_methods.push(quote! {
                fn #method_ident(&self) -> & #manage_ty;
            });
        }
    }

    quote! {
        #(#ordered_parent_of_impls)*
        #(#unordered_parent_of_impls)*
        #(#batch_parent_of_impls)*
        #(#singleton_parent_of_impls)*
        #(#singleton_family_parent_of_impls)*

        pub trait #repo_name: ::std::marker::Send + ::std::marker::Sync {
            #(#ordered_root_manage_methods)*
            #(#unordered_root_manage_methods)*
            #(#batch_root_manage_methods)*
            #(#singleton_root_manage_methods)*
            #(#singleton_family_root_manage_methods)*
            #(#ordered_child_manage_methods)*
            #(#unordered_child_manage_methods)*
            #(#batch_child_manage_methods)*
            #(#singleton_child_manage_methods)*
            #(#singleton_family_child_manage_methods)*
        }
    }
}

fn method_ident_for(prefix: &str, ident: &Ident) -> Ident {
    let snake = to_snake_case(&ident.to_string());
    let name = format!("{}_{}", prefix, snake);
    Ident::new(&name, ident.span())
}

#[derive(Copy, Clone)]
enum CollectionKind {
    Ordered,
    Unordered,
    Batch,
    Singleton,
    SingletonFamily,
}

fn root_manage_ty(kind: CollectionKind, has_children: bool, ty_ident: &Ident) -> TokenStream {
    match (kind, has_children) {
        (CollectionKind::Ordered, false) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootOrdered<#ty_ident> }
        }
        (CollectionKind::Ordered, true) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootOrderedWithChildren<#ty_ident> }
        }
        (CollectionKind::Unordered, false) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootUnordered<#ty_ident> }
        }
        (CollectionKind::Unordered, true) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootUnorderedWithChildren<#ty_ident> }
        }
        (CollectionKind::Batch, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootBatch<#ty_ident> }
        }
        (CollectionKind::Singleton, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootSingleton<#ty_ident> }
        }
        (CollectionKind::SingletonFamily, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageRootSingletonFamily<#ty_ident> }
        }
    }
}

fn child_manage_ty(kind: CollectionKind, has_children: bool, ty_ident: &Ident) -> TokenStream {
    match (kind, has_children) {
        (CollectionKind::Ordered, false) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildOrdered<#ty_ident> }
        }
        (CollectionKind::Ordered, true) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildOrderedWithChildren<#ty_ident> }
        }
        (CollectionKind::Unordered, false) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildUnordered<#ty_ident> }
        }
        (CollectionKind::Unordered, true) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildUnorderedWithChildren<#ty_ident> }
        }
        (CollectionKind::Batch, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildBatch<#ty_ident> }
        }
        (CollectionKind::Singleton, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildSingleton<#ty_ident> }
        }
        (CollectionKind::SingletonFamily, _) => {
            quote! { ::fractic_aws_dynamo::ext::crud::ManageChildSingletonFamily<#ty_ident> }
        }
    }
}
