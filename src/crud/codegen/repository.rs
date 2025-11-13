use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{crud::model::ConfigModel, helpers::to_snake_case};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;

    // Methods for roots.
    let root_methods = model.roots.iter().map(|root| {
        let type_ident = &root.name;
        let method_ident = method_ident_for("manage", &root.name);
        let has_children = !root.children.is_empty() || !root.batch_children.is_empty();
        if has_children {
            quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageRootWithChildren<#type_ident>;
            }
        } else {
            quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageRoot<#type_ident>;
            }
        }
    });

    // Methods for children.
    let child_methods = model.children.iter().map(|child| {
        let type_ident = &child.name;
        let parent_ident = &child.parent;
        let method_ident = method_ident_for("manage", &child.name);
        let has_children = !child.children.is_empty() || !child.batch_children.is_empty();
        if has_children {
            quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageChildWithChildren<#type_ident, Parent = #parent_ident>;
            }
        } else {
            quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageChild<#type_ident, Parent = #parent_ident>;
            }
        }
    });

    // Methods for batch children.
    let batch_methods = model.batches.iter().map(|batch| {
        let type_ident = &batch.name;
        let parent_ident = &batch.parent;
        let method_ident = method_ident_for("manage", &batch.name);
        quote! {
            fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#type_ident, Parent = #parent_ident>;
        }
    });

    quote! {
        pub trait #repo_name: ::std::marker::Send + ::std::marker::Sync {
            #(#root_methods)*
            #(#child_methods)*
            #(#batch_methods)*
        }
    }
}

fn method_ident_for(prefix: &str, ident: &Ident) -> Ident {
    let snake = to_snake_case(&ident.to_string());
    let name = format!("{}_{}", prefix, snake);
    Ident::new(&name, ident.span())
}
