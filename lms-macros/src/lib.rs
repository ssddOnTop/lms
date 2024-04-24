extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn include_path(_input: TokenStream) -> TokenStream {
    let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let path = format!("{}/../html/", cargo_dir);

    // Generate the token stream for the literal path
    let expanded = quote! {
        #path
    };

    TokenStream::from(expanded)
}
