use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{crud::model::ConfigModel, helpers::to_snake_case};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;

    // Methods for roots.
    let root_manage_methods = model.root_objects.iter().map(|root| {
        let type_ident = &root.name;
        let method_ident = method_ident_for("manage", &root.name);
        if root.has_children() {
            quote! {
                fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageRootWithChildren<#type_ident>;
            }
        } else {
            quote! {
                fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageRoot<#type_ident>;
            }
        }
    });

    // Methods for ordered children.
    let (ordered_parent_of_impls, ordered_manage_methods) = model.ordered_objects.iter().map(|child| {
        let type_ident = &child.name;
        let method_ident = method_ident_for("manage", &child.name);
        let parent_of_impls = child.parents.iter().map(|parent_ident| {
            quote! {
                impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
            }
        });
        (
            quote! {
                #(#parent_of_impls)*
            },
            if child.has_children() {
                quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren<#type_ident>;
                }
            } else {
                quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageOrderedChild<#type_ident>;
                }
            }
        )
    }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    // Methods for unordered children.
    let (unordered_parent_of_impls, unordered_manage_methods) = model.unordered_objects.iter().map(|child| {
        let type_ident = &child.name;
        let method_ident = method_ident_for("manage", &child.name);
        let parent_of_impls = child.parents.iter().map(|parent_ident| {
            quote! {
                impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
            }
        });
        (
            quote! {
                #(#parent_of_impls)*
            },
            if child.has_children() {
                quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#type_ident>;
                }
            } else {
                quote! {
                    fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#type_ident>;
                }
            }
        )
    }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    // Methods for batch children.
    let (batch_parent_of_impls, batch_manage_methods) = model.batch_objects.iter().map(|child| {
        let type_ident = &child.name;
        let method_ident = method_ident_for("manage", &child.name);
        let parent_of_impls = child.parents.iter().map(|parent_ident| {
            quote! {
                impl ::fractic_aws_dynamo::ext::crud::ParentOf<#type_ident> for #parent_ident { }
            }
        });
        (
            quote! {
                #(#parent_of_impls)*
            },
            quote! {
                fn #method_ident(&self) -> & ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#type_ident>;
            }
        )
    }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    quote! {
        #(#ordered_parent_of_impls)*
        #(#unordered_parent_of_impls)*
        #(#batch_parent_of_impls)*

        pub trait #repo_name: ::std::marker::Send + ::std::marker::Sync {
            #(#root_manage_methods)*
            #(#ordered_manage_methods)*
            #(#unordered_manage_methods)*
            #(#batch_manage_methods)*
        }
    }
}

fn method_ident_for(prefix: &str, ident: &Ident) -> Ident {
    let snake = to_snake_case(&ident.to_string());
    let name = format!("{}_{}", prefix, snake);
    Ident::new(&name, ident.span())
}
