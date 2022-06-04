use proc_macro::TokenStream;
use quote::ToTokens;
use syn::visit::Visit;

#[derive(Debug)]
struct Visitor {
    generics: Vec<String>,
    result: Vec<syn::TypePath>,
}

impl Visitor {
    fn new(generics: Vec<String>) -> Self {
        Self {
            generics,
            result: vec![],
        }
    }
}

impl<'ast> syn::visit::Visit<'ast> for Visitor {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        if node.path.segments.len() >= 2 {
            if let Some(path) = node.path.segments.first() {
                let path_str = path.ident.to_string();
                if self.generics.contains(&path_str) {
                    self.result.push(node.clone());
                }
            }
        }
        syn::visit::visit_type_path(self, node);
    }
}

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let token = syn::parse_macro_input!(input as syn::DeriveInput);
    match expand(token) {
        Ok(token_stream) => token_stream.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

fn expand(mut st: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let token = impl_debug_for_struct(&mut st)?;
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

fn get_inner_type(ty: &syn::Type, is_on_first: bool) -> Vec<String> {
    let mut res = vec![];
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        if let Some(t) = segments.last() {
            if let syn::PathArguments::None = t.arguments {
                if !is_on_first {
                    res.push(t.ident.to_string());
                }
            } else if let syn::PathArguments::AngleBracketed(
                syn::AngleBracketedGenericArguments { args, .. },
            ) = &t.arguments
            {
                for item in args {
                    if let syn::GenericArgument::Type(t) = item {
                        res.extend(get_inner_type(t, false))
                    }
                }
            }
        }
    }
    res
}

fn get_real_name_of_type(ty: &syn::Type) -> String {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        if let Some(seg) = segments.last() {
            return seg.ident.to_string();
        }
    }
    "".to_string()
}

fn only_in_phantom<'a>(ty: String, tys: &Vec<&syn::Type>) -> bool {
    for item in tys {
        let res = get_inner_type(item, true);
        let type_str = get_real_name_of_type(item);
        if type_str == ty {
            return false;
        }
        if res.contains(&ty) && "PhantomData" != type_str {
            return false;
        }
    }
    true
}

fn get_associat_type(st: &syn::DeriveInput) -> Vec<syn::TypePath> {
    let generics = st
        .generics
        .type_params()
        .map(|item| item.ident.to_string())
        .collect();
    let mut path_visitor = Visitor::new(generics);
    path_visitor.visit_derive_input(st);
    path_visitor.result
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

fn get_custom_bounds(st: &syn::DeriveInput) -> syn::Result<Option<Vec<String>>> {
    let attrs = &st.attrs;
    let mut res = vec![];
    for item in attrs {
        let meta = item.parse_meta()?;
        if let syn::Meta::List(syn::MetaList {
            path: syn::Path { segments, .. },
            nested,
            ..
        }) = meta
        {
            if let Some(seg) = segments.first() {
                if seg.ident.to_string() == "debug" {
                    res = nested
                        .iter()
                        .filter_map(|item| {
                            if let syn::NestedMeta::Meta(syn::Meta::NameValue(
                                syn::MetaNameValue {
                                    path: syn::Path { segments, .. },
                                    lit: syn::Lit::Str(litstr),
                                    ..
                                },
                            )) = item
                            {
                                if let Some(seg) = segments.last() {
                                    if seg.ident.to_string() == "bound" {
                                        return Some(litstr.value().to_string());
                                    }
                                }
                            }
                            None
                        })
                        .collect();
                }
            }
        }
    }
    if res.is_empty() {
        Ok(None)
    } else {
        Ok(Some(res))
    }
}

fn impl_debug_for_struct(st: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
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

    let bounds = get_custom_bounds(st)?;
    if let Some(bounds) = bounds {
        let s = bounds.join(",");
        let mut bounds_str = "where ".to_string();
        bounds_str.push_str(&s);
        let where_clause = syn::WhereClause::from(syn::parse_str(&bounds_str)?);
        let generics = st.generics.clone();
        let (a, b, _) = generics.split_for_impl();
        let token = quote::quote!(
            impl #a std::fmt::Debug for #struct_name #b #where_clause{
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result{
                    f.debug_struct(#struct_name_str)#fields.finish()
                }
            }
        );
        return Ok(token);
    }

    let mut generics = trait_bound_for_generics(st)?;
    let res = get_associat_type(st);
    for item in res {
        generics
            .make_where_clause()
            .predicates
            .push(syn::parse_quote!(#item:std::fmt::Debug));
    }
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
