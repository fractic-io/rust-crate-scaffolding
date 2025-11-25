use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{GenericArgument, Lifetime, PathArguments, Type, TypeParamBound};

use crate::{
    helpers::{to_pascal_case, to_snake_case},
    repository::model::{ConfigModel, FieldSpec, ValueModel},
};

pub fn generate(model: &ConfigModel) -> TokenStream {
    let repo_name = &model.repository_name;
    let repo_name_snake = to_snake_case(&repo_name.to_string());
    let macro_name_ident = Ident::new(
        &format!("generate_{}_handlers", repo_name_snake),
        repo_name.span(),
    );

    let per_fn_handlers: Vec<TokenStream> = model
        .functions
        .iter()
        .map(|f| {
            let fn_ident = &f.name;
            let handler_ident = format_ident!("{}_handler", fn_ident);
            let fn_name = fn_ident.to_string();
            let base_pascal = to_pascal_case(&fn_name);
            let input_ident: Ident = format_ident!("{}Input", base_pascal);
            let output_ident: Ident = format_ident!("{}Output", base_pascal);

            // Handler signature inputs and call-time argument expressions.
            let (handler_params_sig, call_args_ts) = build_handler_inputs(&f.input, &input_ident);

            // Return type for the handler.
            let (handler_ret_ty, map_ok_expr) =
                build_handler_output(&f.output, &output_ident, f.is_direct);

            // Blocking vs async handler.
            let maybe_async = if f.is_blocking {
                quote! {}
            } else {
                quote! { async }
            };
            let maybe_await = if f.is_blocking {
                quote! {}
            } else {
                quote! { .await }
            };

            // Deprecation attribute, if any.
            let maybe_deprecated_attr = if f.is_deprecated {
                if let Some(note) = &f.deprecated_note {
                    quote! { #[deprecated(note = #note)] }
                } else {
                    quote! { #[deprecated] }
                }
            } else {
                quote! {}
            };

            // Compose call site and mapping.
            let call_invoke = quote! {
                let __repo = { __repo_init!() };
                let __result = __repo.#fn_ident(#call_args_ts) #maybe_await;
            };

            let body_ts = if f.is_direct {
                // Direct: no Result wrapper in handler return.
                match &f.output {
                    ValueModel::None => {
                        quote! {
                            #call_invoke
                            ()
                        }
                    }
                    ValueModel::SingleType { .. } => {
                        quote! {
                            #call_invoke
                            __result
                        }
                    }
                    ValueModel::Struct { fields, .. } => {
                        if fields.len() == 1 {
                            let field_name = &fields[0].name;
                            quote! {
                                #call_invoke
                                #output_ident { #field_name: __result }
                            }
                        } else {
                            quote! {
                                #call_invoke
                                __result
                            }
                        }
                    }
                }
            } else {
                // Non-direct: handler returns Result<..., ServerError>
                match &f.output {
                    ValueModel::None => {
                        quote! {
                            #call_invoke
                            __result
                        }
                    }
                    _ => {
                        if let Some(map_expr) = map_ok_expr {
                            quote! {
                                #call_invoke
                                __result.map(#map_expr)
                            }
                        } else {
                            quote! {
                                #call_invoke
                                __result
                            }
                        }
                    }
                }
            };

            quote! {
                #maybe_deprecated_attr
                pub #maybe_async fn #handler_ident(#handler_params_sig) -> #handler_ret_ty {
                    #body_ts
                }
            }
        })
        .collect();

    // The macro accepts a single block/expression that initializes or retrieves the repo.
    // We wrap it in an inner macro so every handler can reuse it without re-parsing.
    let handlers_iter = per_fn_handlers.iter();
    quote! {
        #[allow(unused_macros)]
        macro_rules! #macro_name_ident {
            ($($repo_init:tt)+) => {
                macro_rules! __repo_init { () => { { $($repo_init)+ } } }
                #(#handlers_iter)*
            };
        }

        #[allow(unused_imports)]
        pub(crate) use #macro_name_ident;
    }
}

fn build_handler_inputs(
    input: &ValueModel,
    input_struct_ident: &Ident,
) -> (TokenStream, TokenStream) {
    match input {
        ValueModel::None => (quote! {}, quote! {}),
        ValueModel::SingleType { ty_tokens } => {
            // Accept a serde-friendly type for the handler parameter.
            let serde_ty = adjust_struct_field_lifetimes(ty_tokens.clone());
            let needs_ref_mode = argument_needs_reference(ty_tokens.clone());
            let sig = quote! { input: #serde_ty };
            let call = if needs_ref_mode.requires_ref {
                if needs_ref_mode.original_had_explicit_lifetime {
                    // Serde type kept as a reference (&'static _), pass as-is.
                    quote! { input }
                } else {
                    // Serde type stripped the top-level ref, borrow it.
                    quote! { &input }
                }
            } else {
                quote! { input }
            };
            (sig, call)
        }
        ValueModel::Struct { fields, .. } => {
            let sig = quote! { input: #input_struct_ident };
            let call_args = fields.iter().map(|f: &FieldSpec| {
                let name = &f.name;
                let needs_ref_mode = argument_needs_reference(f.ty_tokens.clone());
                if needs_ref_mode.requires_ref {
                    if needs_ref_mode.original_had_explicit_lifetime {
                        quote! { input.#name }
                    } else {
                        quote! { &input.#name }
                    }
                } else {
                    quote! { input.#name }
                }
            });
            (sig, quote! { #(#call_args),* })
        }
    }
}

fn build_handler_output(
    output: &ValueModel,
    output_struct_ident: &Ident,
    is_direct: bool,
) -> (TokenStream, Option<TokenStream>) {
    fn wrap(is_direct: bool, ty: TokenStream) -> TokenStream {
        if is_direct {
            quote! { #ty }
        } else {
            quote! { ::std::result::Result<#ty, ::fractic_server_error::ServerError> }
        }
    }
    match output {
        ValueModel::None => (wrap(is_direct, quote! { () }), None),
        ValueModel::SingleType { ty_tokens } => {
            // For single type outputs, return the type directly (serde assumed).
            (wrap(is_direct, quote! { #ty_tokens }), None)
        }
        ValueModel::Struct { fields, .. } => {
            if fields.len() == 1 {
                let out_ty = quote! { #output_struct_ident };
                // When repository returns the inner field directly, map Ok(inner) -> Ok(Output { field: inner }).
                let field_name = &fields[0].name;
                let mapper = quote! { |__val| #output_struct_ident { #field_name: __val } };
                (wrap(is_direct, out_ty), Some(mapper))
            } else {
                let out_ty = quote! { #output_struct_ident };
                (wrap(is_direct, out_ty), None)
            }
        }
    }
}

struct RefMode {
    requires_ref: bool,
    original_had_explicit_lifetime: bool,
}

fn argument_needs_reference(tokens: TokenStream) -> RefMode {
    if let Ok(ty) = syn::parse2::<Type>(tokens) {
        match ty {
            Type::Reference(mut r) => {
                let had_lifetime = r.lifetime.take().is_some();
                RefMode {
                    requires_ref: true,
                    original_had_explicit_lifetime: had_lifetime,
                }
            }
            _ => RefMode {
                requires_ref: false,
                original_had_explicit_lifetime: false,
            },
        }
    } else {
        RefMode {
            requires_ref: false,
            original_had_explicit_lifetime: false,
        }
    }
}

/// Parse and normalize a type used in a generated serde struct field (or single input):
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

/// Construct a `syn::Lifetime` from a string like `"'static"`.
fn lifetime_named(name: &str) -> Lifetime {
    Lifetime::new(name, proc_macro2::Span::call_site())
}

/// Target domain for lifetime rewriting.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LifetimeTarget {
    /// Serde struct field position: rewrite explicit `'a`/`'_' to `'static`.
    SerdeStructField,
}

/// Single traversal that rewrites lifetimes across a `syn::Type` according to
/// `target`. When `target` is `SerdeStructField`, explicit `'a`/`'_' lifetimes become `'static`.
fn rewrite_lifetimes_in_type(ty: &mut Type, target: LifetimeTarget, needs_a: &mut bool) {
    match ty {
        Type::Reference(r) => {
            if let Some(l) = &mut r.lifetime {
                if is_lifetime_a_or_underscore(l) {
                    *l = lifetime_named("'static");
                }
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
                                    *l = lifetime_named("'static");
                                }
                            }
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
                        *l = lifetime_named("'static");
                    }
                }
            }
        }
        Type::ImplTrait(it) => {
            for b in it.bounds.iter_mut() {
                if let TypeParamBound::Lifetime(l) = b {
                    if is_lifetime_a_or_underscore(l) {
                        *l = lifetime_named("'static");
                    }
                }
            }
        }
        _ => {}
    }
}

/// True if the lifetime is spelled `'a` or the placeholder `'_'.
fn is_lifetime_a_or_underscore(l: &Lifetime) -> bool {
    let ident = l.ident.to_string();
    ident == "a" || ident == "_"
}
