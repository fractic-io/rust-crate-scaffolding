use proc_macro2::TokenStream;
use quote::quote;

use crate::crud::model::ConfigModel;

pub fn generate(_model: &ConfigModel) -> TokenStream {
    quote! {}
}
