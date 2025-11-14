use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream as TokenStream2, TokenTree};
use syn::{Attribute, Error, Result};

use crate::{helpers::to_pascal_case, repository::ast};

#[derive(Debug)]
pub struct ConfigModel {
    pub repository_name: Ident,
    pub helper_structs: Vec<HelperStruct>,
    pub functions: Vec<FunctionModel>,
}

#[derive(Debug)]
pub struct FunctionModel {
    pub name: Ident,
    pub input: ValueModel,
    pub output: ValueModel,
    pub is_blocking: bool,
    pub is_direct: bool,
}

#[derive(Debug)]
pub enum ValueModel {
    None,
    SingleType {
        /// Verbatim tokens representing the type.
        ty_tokens: TokenStream2,
    },
    Struct {
        /// Flattened list of fields, with helper replacements in types and all
        /// attributes preserved for later codegen.
        fields: Vec<FieldSpec>,
    },
}

#[derive(Debug)]
pub struct HelperStruct {
    pub name: Ident,
    /// Flattened list of fields in this helper, for downstream codegen.
    pub fields: Vec<FieldSpec>,
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
    pub attrs: Vec<Attribute>,
    pub name: Ident,
    pub ty_tokens: TokenStream2,
}

impl TryFrom<ast::ConfigAst> for ConfigModel {
    type Error = Error;

    fn try_from(value: ast::ConfigAst) -> Result<Self> {
        let mut helper_structs: Vec<HelperStruct> = Vec::new();
        let functions = value
            .functions
            .into_iter()
            .map(|f| build_function_model(&value.repository_name, f, &mut helper_structs))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            repository_name: value.repository_name,
            helper_structs,
            functions,
        })
    }
}

fn build_function_model(
    _repo_name: &Ident,
    func: ast::FunctionAst,
    helper_structs: &mut Vec<HelperStruct>,
) -> Result<FunctionModel> {
    let fn_name = func.name.clone();
    let input = build_value_model(&fn_name, None, func.input, helper_structs)?;
    let output = build_value_model(&fn_name, None, func.output, helper_structs)?;
    let (is_blocking, is_direct) = match func.kind {
        ast::FunctionKindAst::Async => (false, false),
        ast::FunctionKindAst::AsyncDirect => (false, true),
        ast::FunctionKindAst::Blocking => (true, false),
        ast::FunctionKindAst::BlockingDirect => (true, true),
    };
    Ok(FunctionModel {
        name: fn_name,
        input,
        output,
        is_blocking,
        is_direct,
    })
}

fn build_value_model(
    fn_name: &Ident,
    chain: Option<Vec<Ident>>,
    val: ast::ValueAst,
    helper_structs: &mut Vec<HelperStruct>,
) -> Result<ValueModel> {
    match val {
        ast::ValueAst::None => Ok(ValueModel::None),
        ast::ValueAst::TypeTokens(ts) => {
            let replaced = replace_inline_structs_in_tokens(
                fn_name,
                chain.as_deref().unwrap_or(&[]),
                ts,
                helper_structs,
            )?;
            Ok(ValueModel::SingleType {
                ty_tokens: replaced,
            })
        }
        ast::ValueAst::Struct(s) => {
            let fields = resolve_inline_struct_fields(fn_name, &[], s, helper_structs)?;
            Ok(ValueModel::Struct { fields })
        }
    }
}

/// Resolve helper structs appearing within an inline struct's fields and
/// produce flattened field specs (attributes preserved).
fn resolve_inline_struct_fields(
    fn_name: &Ident,
    parent_chain: &[Ident],
    s: ast::InlineStructAst,
    helper_structs: &mut Vec<HelperStruct>,
) -> Result<Vec<FieldSpec>> {
    let mut out_fields: Vec<FieldSpec> = Vec::new();
    for field in s.fields {
        let mut chain = parent_chain.to_vec();
        chain.push(field.name.clone());
        let ty_tokens = replace_inline_structs_in_tokens(
            fn_name,
            &chain,
            field.ty_tokens.clone(),
            helper_structs,
        )?;
        out_fields.push(FieldSpec {
            attrs: field.attrs,
            name: field.name,
            ty_tokens,
        });
    }
    Ok(out_fields)
}

/// Walk arbitrary type tokens to find brace-delimited inline structs, convert
/// them to helper structs, and replace them with an Ident of the helper name.
fn replace_inline_structs_in_tokens(
    fn_name: &Ident,
    chain: &[Ident],
    tokens: TokenStream2,
    helper_structs: &mut Vec<HelperStruct>,
) -> Result<TokenStream2> {
    let mut out = TokenStream2::new();
    let mut iter = tokens.into_iter().peekable();
    while let Some(tt) = iter.next() {
        match tt {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                // This is an inline struct at this position.
                // Parse fields from the group's stream.
                let inline: ast::InlineStructAst = syn::parse2(g.stream())?;
                // Resolve nested within this struct first to a flat field list.
                let fields = resolve_inline_struct_fields(fn_name, chain, inline, helper_structs)?;
                // Create helper struct name using fn_name + chain.
                let helper_ident = build_helper_ident(fn_name, chain);
                // Record helper.
                helper_structs.push(HelperStruct {
                    name: helper_ident.clone(),
                    fields,
                });
                // Replace the group with the helper ident token.
                out.extend([TokenTree::Ident(helper_ident)]);
            }
            TokenTree::Group(g)
                if g.delimiter() == Delimiter::Parenthesis
                    || g.delimiter() == Delimiter::Bracket =>
            {
                // Recurse into group.
                let inner =
                    replace_inline_structs_in_tokens(fn_name, chain, g.stream(), helper_structs)?;
                let mut new_group = Group::new(g.delimiter(), inner);
                new_group.set_span(g.span());
                out.extend([TokenTree::Group(new_group)]);
            }
            other => {
                out.extend([other]);
            }
        }
    }
    Ok(out)
}

fn build_helper_ident(fn_name: &Ident, chain: &[Ident]) -> Ident {
    let mut name = to_pascal_case(&fn_name.to_string());
    for c in chain {
        name.push_str(&to_pascal_case(&c.to_string()));
    }
    Ident::new(&name, Span::call_site())
}
