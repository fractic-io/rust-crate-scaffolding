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
        let has_children = !root.ordered_children.is_empty()
            || !root.unordered_children.is_empty()
            || !root.batch_children.is_empty();
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

    // Methods for ordered children.
    let ordered_child_methods = model.ordered_children.iter().flat_map(|child| {
        let type_ident = &child.name;
        let has_children = !child.ordered_children.is_empty()
            || !child.unordered_children.is_empty()
            || !child.batch_children.is_empty();
        if child.parents.len() == 1 {
            let method_ident = method_ident_for("manage", &child.name);
            let p = &child.parents[0];
            if has_children {
                Some(quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren<#type_ident, Parent = #p>;
                })
            } else {
                Some(quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChild<#type_ident, Parent = #p>;
                })
            }
        } else {
            // Generate one method per parent type.
            let methods = child.parents.iter().map(|p| {
                let method_ident = Ident::new(
                    &format!("{}_for_{}", method_ident_for("manage", &child.name), to_snake_case(&p.to_string())),
                    child.name.span(),
                );
                if has_children {
                    quote! {
                        fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren<#type_ident, Parent = #p>;
                    }
                } else {
                    quote! {
                        fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChild<#type_ident, Parent = #p>;
                    }
                }
            });
            Some(quote! { #(#methods)* })
        }
    });

    // Methods for unordered children.
    let unordered_child_methods = model.unordered_children.iter().flat_map(|child| {
        let type_ident = &child.name;
        let has_children = !child.ordered_children.is_empty()
            || !child.unordered_children.is_empty()
            || !child.batch_children.is_empty();
        if child.parents.len() == 1 {
            let method_ident = method_ident_for("manage", &child.name);
            let p = &child.parents[0];
            if has_children {
                Some(quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#type_ident, Parent = #p>;
                })
            } else {
                Some(quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#type_ident, Parent = #p>;
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
                        fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#type_ident, Parent = #p>;
                    }
                } else {
                    quote! {
                        fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#type_ident, Parent = #p>;
                    }
                }
            });
            Some(quote! { #(#methods)* })
        }
    });

    // Methods for batch children.
    let batch_methods = model.batches.iter().flat_map(|batch| {
        let type_ident = &batch.name;
        if batch.parents.len() == 1 {
            let method_ident = method_ident_for("manage", &batch.name);
            let p = &batch.parents[0];
            Some(quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#type_ident, Parent = #p>;
            })
        } else {
            let methods = batch.parents.iter().map(|p| {
                let method_ident = Ident::new(
                    &format!("{}_for_{}", method_ident_for("manage", &batch.name), to_snake_case(&p.to_string())),
                    batch.name.span(),
                );
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#type_ident, Parent = #p>;
                }
            });
            Some(quote! { #(#methods)* })
        }
    });

    quote! {
        pub trait #repo_name: ::std::marker::Send + ::std::marker::Sync {
            #(#root_methods)*
            #(#ordered_child_methods)*
            #(#unordered_child_methods)*
            #(#batch_methods)*
        }
    }
}

fn method_ident_for(prefix: &str, ident: &Ident) -> Ident {
    let snake = to_snake_case(&ident.to_string());
    let name = format!("{}_{}", prefix, snake);
    Ident::new(&name, ident.span())
}
