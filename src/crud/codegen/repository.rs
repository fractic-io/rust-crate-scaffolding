use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{crud::model::ConfigModel, helpers::to_snake_case};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;

    // For children that support multiple parent types, generate a per-child Parent trait
    // and implement it for each allowed parent type. This enables using a single
    // dyn-compatible manager accepting any of the permitted parents.
    let mut emitted_traits: ::std::collections::HashSet<String> =
        ::std::collections::HashSet::new();
    // Work around type constraints by building explicitly for children and batches:
    let mut multi_parent_traits_acc: ::std::vec::Vec<TokenStream> = ::std::vec::Vec::new();
    for child in &model.ordered_children {
        if child.parents.len() > 1 {
            let key = child.name.to_string();
            if emitted_traits.insert(key) {
                let trait_ident =
                    syn::Ident::new(&format!("{}Parent", child.name), child.name.span());
                let impls = child.parents.iter().map(|p| {
                    quote! { impl #trait_ident for #p {} }
                });
                multi_parent_traits_acc.push(quote! {
                    pub trait #trait_ident: ::fractic_aws_dynamo::schema::DynamoObject + ::std::marker::Send + ::std::marker::Sync {}
                    #(#impls)*
                });
            }
        }
    }
    for child in &model.unordered_children {
        if child.parents.len() > 1 {
            let key = child.name.to_string();
            if emitted_traits.insert(key) {
                let trait_ident =
                    syn::Ident::new(&format!("{}Parent", child.name), child.name.span());
                let impls = child.parents.iter().map(|p| {
                    quote! { impl #trait_ident for #p {} }
                });
                multi_parent_traits_acc.push(quote! {
                    pub trait #trait_ident: ::fractic_aws_dynamo::schema::DynamoObject + ::std::marker::Send + ::std::marker::Sync {}
                    #(#impls)*
                });
            }
        }
    }
    for batch in &model.batches {
        if batch.parents.len() > 1 {
            let key = batch.name.to_string();
            if emitted_traits.insert(key) {
                let trait_ident =
                    syn::Ident::new(&format!("{}Parent", batch.name), batch.name.span());
                let impls = batch.parents.iter().map(|p| {
                    quote! { impl #trait_ident for #p {} }
                });
                multi_parent_traits_acc.push(quote! {
                    pub trait #trait_ident: ::fractic_aws_dynamo::schema::DynamoObject + ::std::marker::Send + ::std::marker::Sync {}
                    #(#impls)*
                });
            }
        }
    }

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
    let ordered_child_methods = model.ordered_children.iter().map(|child| {
        let type_ident = &child.name;
        let parent_single = child.parents.len() == 1;
        let parent_ident = if parent_single {
            Some(child.parents[0].clone())
        } else {
            None
        };
        let parent_trait_ident = if parent_single {
            None
        } else {
            Some(syn::Ident::new(&format!("{}Parent", child.name), child.name.span()))
        };
        let method_ident = method_ident_for("manage", &child.name);
        let has_children = !child.ordered_children.is_empty()
            || !child.unordered_children.is_empty()
            || !child.batch_children.is_empty();
        if has_children {
            if parent_single {
                let p = parent_ident.unwrap();
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren<#type_ident, Parent = #p>;
                }
            } else {
                let pt = parent_trait_ident.unwrap();
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren<#type_ident, Parent = dyn #pt>;
                }
            }
        } else {
            if parent_single {
                let p = parent_ident.unwrap();
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChild<#type_ident, Parent = #p>;
                }
            } else {
                let pt = parent_trait_ident.unwrap();
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChild<#type_ident, Parent = dyn #pt>;
                }
            }
        }
    });

    // Methods for unordered children.
    let unordered_child_methods = model.unordered_children.iter().map(|child| {
        let type_ident = &child.name;
        let parent_single = child.parents.len() == 1;
        let parent_ident = if parent_single {
            Some(child.parents[0].clone())
        } else {
            None
        };
        let parent_trait_ident = if parent_single {
            None
        } else {
            Some(syn::Ident::new(&format!("{}Parent", child.name), child.name.span()))
        };
        let method_ident = method_ident_for("manage", &child.name);
        let has_children = !child.ordered_children.is_empty()
            || !child.unordered_children.is_empty()
            || !child.batch_children.is_empty();
        if has_children {
            if parent_single {
                let p = parent_ident.unwrap();
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#type_ident, Parent = #p>;
                }
            } else {
                let pt = parent_trait_ident.unwrap();
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#type_ident, Parent = dyn #pt>;
                }
            }
        } else {
            if parent_single {
                let p = parent_ident.unwrap();
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#type_ident, Parent = #p>;
                }
            } else {
                let pt = parent_trait_ident.unwrap();
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#type_ident, Parent = dyn #pt>;
                }
            }
        }
    });

    // Methods for batch children.
    let batch_methods = model.batches.iter().map(|batch| {
        let type_ident = &batch.name;
        let parent_single = batch.parents.len() == 1;
        let parent_ident = if parent_single {
            Some(batch.parents[0].clone())
        } else {
            None
        };
        let parent_trait_ident = if parent_single {
            None
        } else {
            Some(syn::Ident::new(&format!("{}Parent", batch.name), batch.name.span()))
        };
        let method_ident = method_ident_for("manage", &batch.name);
        if parent_single {
            let p = parent_ident.unwrap();
            quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#type_ident, Parent = #p>;
            }
        } else {
            let pt = parent_trait_ident.unwrap();
            quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#type_ident, Parent = dyn #pt>;
            }
        }
    });

    quote! {
        #(#multi_parent_traits_acc)*
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
