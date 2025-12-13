use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use super::repository::{ObjectType, child_manage_ty, root_manage_ty};
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
    let mut ordered_fields = Vec::new();
    let mut ordered_inits = Vec::new();
    let mut ordered_trait_impls = Vec::new();
    for ordered in &model.ordered_objects {
        let method_ident = method_ident_for("manage", &ordered.name);
        let ty_ident = &ordered.name;
        let manage_ty = if ordered.parents.is_none() {
            root_manage_ty(ObjectType::Ordered, ordered.has_children(), ty_ident)
        } else {
            child_manage_ty(ObjectType::Ordered, ordered.has_children(), ty_ident)
        };
        gen_field_init_impl(
            &mut ordered_fields,
            &mut ordered_inits,
            &mut ordered_trait_impls,
            &method_ident,
            manage_ty,
        );
    }

    // Fields and initializers for unordered collections.
    let mut unordered_fields = Vec::new();
    let mut unordered_inits = Vec::new();
    let mut unordered_trait_impls = Vec::new();
    for unordered in &model.unordered_objects {
        let method_ident = method_ident_for("manage", &unordered.name);
        let ty_ident = &unordered.name;
        let manage_ty = if unordered.parents.is_none() {
            root_manage_ty(ObjectType::Unordered, unordered.has_children(), ty_ident)
        } else {
            child_manage_ty(ObjectType::Unordered, unordered.has_children(), ty_ident)
        };
        gen_field_init_impl(
            &mut unordered_fields,
            &mut unordered_inits,
            &mut unordered_trait_impls,
            &method_ident,
            manage_ty,
        );
    }

    // Fields, inits, trait impls for batch collections.
    let mut batch_fields = Vec::new();
    let mut batch_inits = Vec::new();
    let mut batch_trait_impls = Vec::new();
    for batch in &model.batch_objects {
        let method_ident = method_ident_for("manage", &batch.name);
        let ty_ident = &batch.name;
        let manage_ty = if batch.parents.is_none() {
            root_manage_ty(ObjectType::Batch, false, ty_ident)
        } else {
            child_manage_ty(ObjectType::Batch, false, ty_ident)
        };
        gen_field_init_impl(
            &mut batch_fields,
            &mut batch_inits,
            &mut batch_trait_impls,
            &method_ident,
            manage_ty,
        );
    }

    // Fields and initializers for singleton roots/children.
    let mut singleton_fields = Vec::new();
    let mut singleton_inits = Vec::new();
    let mut singleton_trait_impls = Vec::new();
    for singleton in &model.singleton_objects {
        let method_ident = method_ident_for("manage", &singleton.name);
        let ty_ident = &singleton.name;
        let manage_ty = if singleton.parents.is_none() {
            root_manage_ty(ObjectType::Singleton, false, ty_ident)
        } else {
            child_manage_ty(ObjectType::Singleton, false, ty_ident)
        };
        gen_field_init_impl(
            &mut singleton_fields,
            &mut singleton_inits,
            &mut singleton_trait_impls,
            &method_ident,
            manage_ty,
        );
    }

    // Fields and initializers for singleton family roots/children.
    let mut singleton_family_fields = Vec::new();
    let mut singleton_family_inits = Vec::new();
    let mut singleton_family_trait_impls = Vec::new();
    for singleton_family in &model.singleton_family_objects {
        let method_ident = method_ident_for("manage", &singleton_family.name);
        let ty_ident = &singleton_family.name;
        let manage_ty = if singleton_family.parents.is_none() {
            root_manage_ty(ObjectType::SingletonFamily, false, ty_ident)
        } else {
            child_manage_ty(ObjectType::SingletonFamily, false, ty_ident)
        };
        gen_field_init_impl(
            &mut singleton_family_fields,
            &mut singleton_family_inits,
            &mut singleton_family_trait_impls,
            &method_ident,
            manage_ty,
        );
    }

    let out = quote! {
        pub struct #impl_struct_ident {
            #(#ordered_fields,)*
            #(#unordered_fields,)*
            #(#batch_fields,)*
            #(#singleton_fields,)*
            #(#singleton_family_fields,)*
        }

        impl #impl_struct_ident {
            pub async fn new(ctx: __ctx!()) -> ::std::result::Result<Self, ::fractic_server_error::ServerError> {
                let dynamo_util = ::std::sync::Arc::new(::fractic_aws_dynamo::util::DynamoUtil::new(ctx, ctx.$ctx_db_method()).await?);
                let crud_algorithms = ::std::sync::Arc::new(<$crud_algorithms>::new(dynamo_util.clone()));
                Ok(Self {
                    #(#ordered_inits,)*
                    #(#unordered_inits,)*
                    #(#batch_inits,)*
                    #(#singleton_inits,)*
                    #(#singleton_family_inits,)*
                })
            }
        }

        impl #repo_name for #impl_struct_ident {
            #(#ordered_trait_impls)*
            #(#unordered_trait_impls)*
            #(#batch_trait_impls)*
            #(#singleton_trait_impls)*
            #(#singleton_family_trait_impls)*
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

fn gen_field_init_impl(
    fields: &mut Vec<TokenStream>,
    inits: &mut Vec<TokenStream>,
    trait_impls: &mut Vec<TokenStream>,
    method_ident: &Ident,
    manage_ty: TokenStream,
) {
    let init_ty = manage_ty.clone();
    let trait_ty = manage_ty.clone();
    fields.push(quote! {
        #method_ident: #manage_ty
    });
    inits.push(quote! {
        #method_ident: <#init_ty>::new(
            dynamo_util.clone(),
            crud_algorithms.clone(),
        )
    });
    trait_impls.push(quote! {
        fn #method_ident(&self) -> & #trait_ty {
            &self.#method_ident
        }
    });
}

fn method_ident_for(prefix: &str, ident: &Ident) -> Ident {
    let snake = to_snake_case(&ident.to_string());
    let name = format!("{}_{}", prefix, snake);
    Ident::new(&name, ident.span())
}
