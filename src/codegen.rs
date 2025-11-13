use crate::model::RepositoryScaffolding;
use proc_macro2::TokenStream;
use quote::quote;

pub fn generate(_model: &RepositoryScaffolding) -> TokenStream {
    // Intentionally empty for now – this module exists to provide a clean seam
    // for multiple codegen backends in the future.
    quote! {}
}
