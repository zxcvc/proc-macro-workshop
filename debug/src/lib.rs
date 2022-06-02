use proc_macro::TokenStream;
use quote::ToTokens;
#[proc_macro_derive(CustomDebug, attributes(debug))]
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

fn get_field_arrtibutes(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::Token![,]>,
) -> impl Iterator<Item = String> + '_ {
    fields.iter().map(|item| {
        let at = &item.attrs;
        let res = at.iter().find_map(|item| {
            let meta = item.parse_meta().unwrap();
            if let syn::Meta::NameValue(syn::MetaNameValue {
                path: syn::Path { segments, .. },
                lit: syn::Lit::Str(lit_str),
                ..
            }) = meta
            {
                return segments.iter().find_map(|it| {
                    if it.ident.to_string() == "debug" {
                        return Some(lit_str.value());
                    }
                    None
                });
            }
            None
        });
        match res {
            Some(value) => value,
            None => "{:?}".to_string(),
        }
    })
}

fn get_field(
    st: &syn::DeriveInput,
) -> syn::Result<&syn::punctuated::Punctuated<syn::Field, syn::Token![,]>> {
    match &st.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => Ok(named),
        _ => Err(syn::Error::new_spanned(&st.ident, "need a struct")),
    }
}

fn get_inner_type(ty: &syn::Type) -> Vec<String> {
    let mut res = vec![];
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        if let Some(t) = segments.last() {
            if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                args,
                ..
            }) = &t.arguments
            {
                let iter = args.iter().map(|it| it.to_token_stream().to_string());
                res.extend(iter);
            }
        }
    }
    res
}

fn only_in_phantom<'a>(ty: String, tys: &Vec<&syn::Type>) -> bool {
    for item in tys {
        let res = get_inner_type(item);
        let type_str = item.to_token_stream().to_string();
        if type_str == ty {
            return false;
        }
        if res.contains(&ty) && (ty == "PhantomData" || ty.ends_with("::PhantomData")) {
            return false;
        }
    }
    true
}

fn trait_bound_for_generics(st: &syn::DeriveInput) -> syn::Result<syn::Generics> {
    let tys: Vec<_> = get_field(st)?.iter().map(|item| &item.ty).collect();

    let mut generics = st.generics.clone();
    for item in generics.params.iter_mut() {
        if let syn::GenericParam::Type(ty) = item {
            if !only_in_phantom(ty.ident.to_string(), &tys) {
                ty.bounds.push(syn::parse_quote!(std::fmt::Debug));
            }
        }
    }
    Ok(generics)
}

fn impl_debug_for_struct(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = get_struct_name(st)?;
    let struct_name_str = struct_name.to_string();
    let fields = get_field(st)?;
    let names = fields.iter().map(|item| &item.ident);
    let names_str = fields
        .iter()
        .map(|item| item.ident.to_token_stream().to_string());
    let arrtibures = get_field_arrtibutes(fields);
    let fields = quote::quote!(
        #(.field(#names_str,&std::format_args!(#arrtibures,&self.#names)))*
    );
    let generics = trait_bound_for_generics(st)?;
    let (a, b, c) = generics.split_for_impl();
    let token = quote::quote!(
        impl #a std::fmt::Debug for #struct_name #b #c{
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result{
                f.debug_struct(#struct_name_str)#fields.finish()
            }
        }
    );
    Ok(token)
}
