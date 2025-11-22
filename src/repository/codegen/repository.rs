use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{GenericArgument, Lifetime, PathArguments, Type, TypeParamBound};

use crate::{
    helpers::to_pascal_case,
    repository::model::{ConfigModel, FieldSpec, ValueModel},
};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let helper_structs = generate_helper_structs(model);
    let (io_structs, trait_methods) = generate_functions_and_trait_methods(model);
    let repo_name = &model.repository_name;

    quote! {
        #helper_structs
        #io_structs

        #[::async_trait::async_trait]
        pub trait #repo_name: ::std::marker::Send + ::std::marker::Sync {
            #(#trait_methods)*
        }
    }
}

fn generate_helper_structs(model: &ConfigModel) -> TokenStream {
    let helpers = model.helper_structs.iter().map(|h| {
        let name = &h.name;
        let fields = generate_struct_fields(&h.fields);
        quote! {
            #[allow(non_camel_case_types)]
            #[derive(::core::clone::Clone, ::core::fmt::Debug, ::serde::Serialize, ::serde::Deserialize)]
            pub struct #name {
                #(#fields),*
            }
        }
    });
    quote! { #(#helpers)* }
}

fn generate_functions_and_trait_methods(model: &ConfigModel) -> (TokenStream, Vec<TokenStream>) {
    let mut io_structs_accum = Vec::new();
    let mut trait_methods = Vec::new();

    for f in &model.functions {
        let fn_ident = &f.name;
        let fn_name = fn_ident.to_string();
        let base_pascal = to_pascal_case(&fn_name);
        let input_ident: Ident = format_ident!("{}Input", base_pascal);
        let output_ident: Ident = format_ident!("{}Output", base_pascal);

        // Define input struct if needed.
        if let ValueModel::Struct { fields } = &f.input {
            let fields_ts = generate_struct_fields(fields);
            io_structs_accum.push(quote! {
                #[derive(::core::clone::Clone, ::core::fmt::Debug, ::serde::Deserialize)]
                pub struct #input_ident {
                    #(#fields_ts),*
                }
            });
        }
        // Define output struct if needed (always define for Struct, even if
        // single field).
        if let ValueModel::Struct { fields } = &f.output {
            let fields_ts = generate_struct_fields(fields);
            io_structs_accum.push(quote! {
                #[derive(::core::clone::Clone, ::core::fmt::Debug, ::serde::Serialize)]
                pub struct #output_ident {
                    #(#fields_ts),*
                }
            });
        }

        // Build input parameters for the trait method.
        let (inputs_ts, needs_a_lifetime) = build_method_inputs(&f.input);

        // Build return type for the trait method.
        let output_ts = build_method_output(&f.output, &output_ident, f.is_direct);

        // Compose method signature.
        let maybe_generics = if needs_a_lifetime {
            quote! { <'a> }
        } else {
            quote! {}
        };
        let maybe_deprecated_attr = if f.is_deprecated {
            if let Some(note) = &f.deprecated_note {
                quote! { #[deprecated(note = #note)] }
            } else {
                quote! { #[deprecated] }
            }
        } else {
            quote! {}
        };
        if f.is_blocking {
            trait_methods.push(quote! {
                #maybe_deprecated_attr
                fn #fn_ident #maybe_generics (&self #inputs_ts) -> #output_ts;
            });
        } else {
            trait_methods.push(quote! {
                #maybe_deprecated_attr
                async fn #fn_ident #maybe_generics (&self #inputs_ts) -> #output_ts;
            });
        }
    }

    (quote! { #(#io_structs_accum)* }, trait_methods)
}

/// Build the trait method inputs and report whether the method must be generic
/// over a lifetime `'a` (i.e., any argument contained an explicit `'a`/`'_'` that
/// was normalized to `'a`).
fn build_method_inputs(input: &ValueModel) -> (TokenStream, bool) {
    match input {
        ValueModel::None => (quote! {}, false),
        ValueModel::SingleType { ty_tokens } => {
            let (normalized, needs) = adjust_argument_lifetimes(ty_tokens.clone());
            (quote! { , input: #normalized }, needs)
        }
        ValueModel::Struct { fields, .. } => {
            let mut needs_any = bool::default();
            let params = fields.iter().map(|f: &FieldSpec| {
                let name = &f.name;
                let (normalized, needs) = adjust_argument_lifetimes(f.ty_tokens.clone());
                if needs {
                    needs_any = true;
                }
                quote! { #name: #normalized }
            });
            (quote! { , #(#params),* }, needs_any)
        }
    }
}

fn build_method_output(
    output: &ValueModel,
    output_struct_ident: &Ident,
    is_direct: bool,
) -> TokenStream {
    fn wrap(is_direct: bool, ty: TokenStream) -> TokenStream {
        if is_direct {
            quote! { #ty }
        } else {
            quote! { ::std::result::Result<#ty, ::fractic_server_error::ServerError> }
        }
    }
    match output {
        ValueModel::None => wrap(is_direct, quote! { () }),
        ValueModel::SingleType { ty_tokens } => wrap(is_direct, quote! { #ty_tokens }),
        ValueModel::Struct { fields, .. } => {
            if fields.len() == 1 {
                let ty = &fields[0].ty_tokens;
                wrap(is_direct, quote! { #ty })
            } else {
                let out = output_struct_ident;
                wrap(is_direct, quote! { #out })
            }
        }
    }
}

fn generate_struct_fields(fields: &[FieldSpec]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f| {
            let attrs = &f.attrs;
            let name = &f.name;
            let ty = adjust_struct_field_lifetimes(f.ty_tokens.clone());
            quote! { #(#attrs)* pub #name: #ty }
        })
        .collect()
}

/// Parse and normalize a type used in a method argument:
/// - References with lifetime `'a` or `'_' are rewritten to `'a`.
/// - References without an explicit lifetime are left unchanged.
///
/// Returns the rewritten tokens and whether the method needs a `'a` generic.
fn adjust_argument_lifetimes(tokens: TokenStream) -> (TokenStream, bool) {
    if let Ok(mut ty) = syn::parse2::<Type>(tokens.clone()) {
        let mut needs_a = false;
        rewrite_lifetimes_in_type(&mut ty, LifetimeTarget::MethodArg, &mut needs_a);
        (quote! { #ty }, needs_a)
    } else {
        (tokens, false)
    }
}

/// Parse and normalize a type used in a generated serde struct field:
/// - Top-level reference without a lifetime is stripped to its inner type.
/// - Any `'a` or `'_' lifetime (at any depth) is rewritten to `'static`.
fn adjust_struct_field_lifetimes(tokens: TokenStream) -> TokenStream {
    if let Ok(mut ty_parsed) = syn::parse2::<Type>(tokens.clone()) {
        // Strip the top-level reference if it has no lifetime.
        let mut ty = if let Type::Reference(r) = &mut ty_parsed {
            if r.lifetime.is_none() {
                (*r.elem).clone()
            } else {
                ty_parsed
            }
        } else {
            ty_parsed
        };
        let mut _unused = false;
        rewrite_lifetimes_in_type(&mut ty, LifetimeTarget::SerdeStructField, &mut _unused);
        quote! { #ty }
    } else {
        tokens
    }
}

/// Construct a `syn::Lifetime` from a string like `"'a"` or `"'static"`.
fn lifetime_named(name: &str) -> Lifetime {
    Lifetime::new(name, proc_macro2::Span::call_site())
}

/// True if the lifetime is spelled `'a` or the placeholder `'_'.
fn is_lifetime_a_or_underscore(l: &Lifetime) -> bool {
    let ident = l.ident.to_string();
    ident == "a" || ident == "_"
}

/// Target domain for lifetime rewriting.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LifetimeTarget {
    /// Method argument position: rewrite explicit `'a`/`'_' to `'a` and signal
    /// if `< 'a >` is required.
    MethodArg,
    /// Serde struct field position: rewrite explicit `'a`/`'_' to `'static`.
    SerdeStructField,
}

/// Single traversal that rewrites lifetimes across a `syn::Type` according to
/// `target`. When `target` is `MethodArg`, explicit `'a`/`'_' lifetimes become
/// `'a` and `needs_a` is set. Missing lifetimes are left unchanged. When
/// `target` is `SerdeStructField`, explicit `'a`/`'_' lifetimes become `'static`.
fn rewrite_lifetimes_in_type(ty: &mut Type, target: LifetimeTarget, needs_a: &mut bool) {
    match ty {
        Type::Reference(r) => {
            match target {
                LifetimeTarget::MethodArg => match &mut r.lifetime {
                    Some(l) => {
                        if is_lifetime_a_or_underscore(l) {
                            *l = lifetime_named("'a");
                            *needs_a = true;
                        }
                    }
                    None => {}
                },
                LifetimeTarget::SerdeStructField => match &mut r.lifetime {
                    Some(l) => {
                        if is_lifetime_a_or_underscore(l) {
                            *l = lifetime_named("'static");
                        }
                    }
                    None => {}
                },
            }
            rewrite_lifetimes_in_type(&mut r.elem, target, needs_a);
        }
        Type::Tuple(t) => {
            for elem in &mut t.elems {
                rewrite_lifetimes_in_type(elem, target, needs_a);
            }
        }
        Type::Slice(s) => {
            rewrite_lifetimes_in_type(&mut s.elem, target, needs_a);
        }
        Type::Array(a) => {
            rewrite_lifetimes_in_type(&mut a.elem, target, needs_a);
        }
        Type::Paren(p) => {
            rewrite_lifetimes_in_type(&mut p.elem, target, needs_a);
        }
        Type::Group(g) => {
            rewrite_lifetimes_in_type(&mut g.elem, target, needs_a);
        }
        Type::Path(p) => {
            for seg in p.path.segments.iter_mut() {
                if let PathArguments::AngleBracketed(ab) = &mut seg.arguments {
                    for arg in ab.args.iter_mut() {
                        match arg {
                            GenericArgument::Type(t) => {
                                rewrite_lifetimes_in_type(t, target, needs_a)
                            }
                            GenericArgument::Lifetime(l) => {
                                if is_lifetime_a_or_underscore(l) {
                                    match target {
                                        LifetimeTarget::MethodArg => {
                                            *l = lifetime_named("'a");
                                            *needs_a = true;
                                        }
                                        LifetimeTarget::SerdeStructField => {
                                            *l = lifetime_named("'static");
                                        }
                                    }
                                }
                            }
                            GenericArgument::Const(_) => {}
                            _ => {}
                        }
                    }
                }
            }
        }
        Type::TraitObject(obj) => {
            for b in obj.bounds.iter_mut() {
                if let TypeParamBound::Lifetime(l) = b {
                    if is_lifetime_a_or_underscore(l) {
                        match target {
                            LifetimeTarget::MethodArg => {
                                *l = lifetime_named("'a");
                                *needs_a = true;
                            }
                            LifetimeTarget::SerdeStructField => {
                                *l = lifetime_named("'static");
                            }
                        }
                    }
                }
            }
        }
        Type::ImplTrait(it) => {
            for b in it.bounds.iter_mut() {
                if let TypeParamBound::Lifetime(l) = b {
                    if is_lifetime_a_or_underscore(l) {
                        match target {
                            LifetimeTarget::MethodArg => {
                                *l = lifetime_named("'a");
                                *needs_a = true;
                            }
                            LifetimeTarget::SerdeStructField => {
                                *l = lifetime_named("'static");
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
