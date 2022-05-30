use proc_macro::TokenStream;
use quote::ToTokens;
#[proc_macro_derive(CustomDebug)]
pub fn derive(input: TokenStream) -> TokenStream {
    let token = syn::parse_macro_input!(input as syn::DeriveInput);
    match expand(token) {
        Ok(token_stream) => token_stream.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

fn expand(st: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let token = impl_debug_for_struct(&st)?;
    Ok(token)
}

fn get_struct_name(st: &syn::DeriveInput) -> syn::Result<syn::Ident> {
    Ok(st.ident.clone())
}

fn get_field(st: &syn::DeriveInput) -> syn::Result<&syn::punctuated::Punctuated<syn::Field,syn::Token![,]>> {
    match &st.data {
        syn::Data::Struct(syn::DataStruct { fields:syn::Fields::Named(syn::FieldsNamed{
            named,..
        }), .. }) => Ok(named),
        _ => Err(syn::Error::new_spanned(&st.ident, "need a struct")),
    }
}

fn impl_debug_for_struct(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = get_struct_name(st)?;
    let struct_name_str = struct_name.to_string();
    let fields = get_field(st)?;
    let names = fields.iter().map(|item|&item.ident);
    let names_str = fields.iter().map(|item|item.ident.to_token_stream().to_string());
    let fields = quote::quote!(
        #(.field(#names_str,&self.#names))*
    );
    let token = quote::quote!(
        impl std::fmt::Debug for #struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result{
                f.debug_struct(#struct_name_str)#fields.finish()
            }
        }
    );
    Ok(token)
}
