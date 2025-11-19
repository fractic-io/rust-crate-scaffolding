use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{crud::model::ConfigModel, helpers::to_snake_case};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let repo_impl_name = Ident::new(&format!("{}Impl", repo_name), repo_name.span());

    // Methods for roots.
    let root_manage_methods = model.roots.iter().map(|root| {
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
    let (ordered_dynamic_parents, ordered_manage_methods) = model.ordered_children.iter().map(|child| {
        let type_ident = &child.name;
        let method_ident = method_ident_for("manage", &child.name);
        if child.parents.len() == 1 {
            let parent_ident = &child.parents[0];
            let manage_method = if child.has_children() {
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChildWithChildren<#type_ident, Parent = #parent_ident>;
                }
            } else {
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageOrderedChild<#type_ident, Parent = #parent_ident>;
                }
            };
            (quote! { }, manage_method)
        } else {
            todo!()
        }
    }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    // Methods for unordered children.
    let (unordered_dynamic_parents, unordered_manage_methods) = model.unordered_children.iter().map(|child| {
        let type_ident = &child.name;
        let method_ident = method_ident_for("manage", &child.name);
        if child.parents.len() == 1 {
            let parent_ident = &child.parents[0];
            let manage_method = if child.has_children() {
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#type_ident, Parent = #parent_ident>;
                }
            } else {
                quote! {
                    fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#type_ident, Parent = #parent_ident>;
                }
            };
            (quote! { }, manage_method)
        } else {
            let dynamic_parent_ident = Ident::new(&format!("{}Parent", type_ident), type_ident.span());
            let (sealed_trait, manage_method) = if child.has_children() {
                let sealed_trait_impls = (child.parents.iter().map(|parent_ident| {
                    let method_ident_for_parent = method_ident_for_with_parent("manage", &method_ident, parent_ident);
                    quote! {
                        impl sealed::#dynamic_parent_ident for #parent_ident {
                            fn resolve<'a>(r: &'a #repo_impl_name) -> &'a dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#type_ident, Parent = Self> {
                                &r.#method_ident_for_parent
                            }
                        }
                        impl #dynamic_parent_ident for #parent_ident {}
                    }
                })).collect::<Vec<_>>();
                let sealed_trait = quote! {
                    mod sealed {
                        pub trait #dynamic_parent_ident : ::std::marker::Sized {
                            fn resolve<'a>(r: &'a #repo_impl_name) -> &'a dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#type_ident, Parent = Self>;
                        }
                    }
                    pub trait #dynamic_parent_ident : sealed::#dynamic_parent_ident {}
                    #(#sealed_trait_impls)*
                };
                (sealed_trait, quote! {
                    fn #method_ident<P: #dynamic_parent_ident>(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChildWithChildren<#type_ident, Parent = P>
                    where
                        Self: Sized;
                })
            } else {
                let sealed_trait_impls = (child.parents.iter().map(|parent_ident| {
                    let method_ident_for_parent = method_ident_for_with_parent("manage", &method_ident, parent_ident);
                    quote! {
                        impl sealed::#dynamic_parent_ident for #parent_ident {
                            fn resolve<'a>(r: &'a #repo_impl_name) -> &'a dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#type_ident, Parent = Self> {
                                &r.#method_ident_for_parent
                            }
                        }
                        impl #parent_ident for #dynamic_parent_ident {}
                    }
                })).collect::<Vec<_>>();
                let sealed_trait = quote! {
                    mod sealed {
                        pub trait #dynamic_parent_ident : ::std::marker::Sized {
                            fn resolve<'a>(r: &'a #repo_impl_name) -> &'a dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#type_ident, Parent = Self>;
                        }
                    }
                    pub trait #dynamic_parent_ident : sealed::#dynamic_parent_ident {}
                    #(#sealed_trait_impls)*
                };
                (sealed_trait, quote! {
                    fn #method_ident<P: #dynamic_parent_ident>(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageUnorderedChild<#type_ident, Parent = P>;
                })
            };
            (sealed_trait, manage_method)
        }
    }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    // Methods for batch children.
    let (batch_dynamic_parents, batch_manage_methods) = model.batches.iter().map(|batch| {
        let type_ident = &batch.name;
        let method_ident = method_ident_for("manage", &batch.name);
        if batch.parents.len() == 1 {
            let parent_ident = &batch.parents[0];
            let manage_method = quote! {
                fn #method_ident(&self) -> &dyn ::fractic_aws_dynamo::ext::crud::ManageBatchChild<#type_ident, Parent = #parent_ident>;
            };
            (quote! { }, manage_method)
        } else {
            todo!()
        }
    }).unzip::<TokenStream, TokenStream, Vec<_>, Vec<_>>();

    quote! {
        #(#ordered_dynamic_parents)*
        #(#unordered_dynamic_parents)*
        #(#batch_dynamic_parents)*

        pub trait #repo_name: ::std::marker::Send + ::std::marker::Sync {
            #(#root_manage_methods)*
            #(#ordered_manage_methods)*
            #(#unordered_manage_methods)*
            #(#batch_manage_methods)*
        }
    }
}

pub fn method_ident_for(prefix: &str, ident: &Ident) -> Ident {
    let snake = to_snake_case(&ident.to_string());
    let name = format!("{}_{}", prefix, snake);
    Ident::new(&name, ident.span())
}

pub fn method_ident_for_with_parent(prefix: &str, ident: &Ident, parent: &Ident) -> Ident {
    let ident_snake = to_snake_case(&ident.to_string());
    let parent_snake = to_snake_case(&parent.to_string());
    let name = format!("{}_{}_for_{}", prefix, ident_snake, parent_snake);
    Ident::new(&name, ident.span())
}
