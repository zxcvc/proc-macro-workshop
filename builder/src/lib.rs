use proc_macro::TokenStream;
use proc_macro2;
use quote;
use syn::spanned::Spanned;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let st = syn::parse_macro_input!(input as syn::DeriveInput);
    match build(st) {
        Ok(token_stream) => token_stream.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn build(st: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let new_struct_ident = build_new_struct(&st)?;
    let impl_for_stuct = impl_for_old_struct(&st)?;
    let impl_for_new_struct_setter = impl_for_new_struct_setter(&st)?;
    let impl_build_for_new_struct = impl_build_for_new_struct(&st)?;
    let ret = quote::quote!(
        #new_struct_ident
        #impl_for_stuct
        #impl_for_new_struct_setter
        #impl_build_for_new_struct
    );
    Ok(ret)
}

type StructFields = syn::punctuated::Punctuated<syn::Field, syn::Token![,]>;

fn get_struct_fields(st: &syn::DeriveInput) -> syn::Result<&StructFields> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = &st.data
    {
        return Ok(named);
    } else {
        Err(syn::Error::new_spanned(
            &st.ident,
            "need a struct,but found a enum",
        ))
    }
}

fn build_new_struct(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = st.ident.to_string();
    let new_struct_name = format!("{}Builder", struct_name);
    let new_struct_ident = syn::Ident::new(&new_struct_name, st.span());
    let struct_vis = &st.vis;
    let fields = get_struct_fields(st)?;
    let mut token_stream = proc_macro2::TokenStream::new();
    for item in fields {
        let field_name = &item.ident;
        let type_name = &item.ty;
        let type_name = match (
            get_inner_type_of_option(&item.ty, "Option"),
            get_inner_type_of_option(&item.ty, "Vec"),
        ) {
            (Some(t), _) => quote::quote!(std::option::Option<#t>),
            (_, Some(t)) => quote::quote!(std::vec::Vec<#t>),
            _ => quote::quote!(std::option::Option<#type_name>),
        };
        let vis = &item.vis;
        token_stream.extend(quote::quote!(
            #vis #field_name:#type_name,
        ));
    }
    let ret = quote::quote!(
        #struct_vis struct #new_struct_ident{
            #token_stream
        }
    );
    Ok(ret)
}

fn impl_for_old_struct(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let old_struct_ident = &st.ident;
    let new_struct_name = format!("{}Builder", old_struct_ident.to_string());
    let new_struct_ident = syn::Ident::new(&new_struct_name, st.span());

    let fields = get_struct_fields(st)?;
    // let names:Vec<_> = fields.iter().map(|item|&item.ident).collect();
    let mut token_stream = proc_macro2::TokenStream::new();
    for item in fields {
        let field_name = &item.ident;
        let type_name = &item.ty;
        let prototype = match (
            get_inner_type_of_option(type_name, "Option"),
            get_inner_type_of_option(type_name, "Vec"),
        ) {
            (Some(_t), _) => quote::quote!(std::option::Option::None,),
            (_, Some(_t)) => quote::quote!(std::vec::Vec::new(),),
            _ => quote::quote!(std::option::Option::None,),
        };
        token_stream.extend(quote::quote!(#field_name:#prototype));
    }

    let ret = quote::quote!(
        impl #old_struct_ident{
            pub fn builder()->#new_struct_ident{
                #new_struct_ident{
                    #token_stream
                }
            }
        }
    );
    Ok(ret)
}

fn impl_for_new_struct_setter(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let new_struct_name = format!("{}Builder", st.ident.to_string());
    let new_struct_ident = syn::Ident::new(&new_struct_name, st.span());

    let fields = get_struct_fields(st)?;
    let mut token_stream = proc_macro2::TokenStream::new();
    for item in fields.iter() {
        let field_name = &item.ident;
        let origin_type_name = &item.ty;
        let (type_name, each_type_name, seeter_body) = match (
            get_inner_type_of_option(&item.ty, "Option"),
            get_inner_type_of_option(&item.ty, "Vec"),
        ) {
            (Some(t), _) => (
                quote::quote!(#t),
                t,
                quote::quote!(self.#field_name = std::option::Option::Some(#field_name)),
            ),
            (_, Some(t)) => (
                quote::quote!(std::vec::Vec<#t>),
                t,
                quote::quote!(self.#field_name = #field_name),
            ),
            _ => (
                quote::quote!(#origin_type_name),
                origin_type_name,
                quote::quote!(self.#field_name = std::option::Option::Some(#field_name)),
            ),
        };
        if let Some(each_ident_name) = get_attributes(item)? {
            if each_ident_name.to_string() == field_name.as_ref().unwrap().to_string() {
            } else {
                token_stream.extend(quote::quote!(
                    fn #each_ident_name(&mut self,#each_ident_name:#each_type_name)->&mut Self{
                        self.#field_name.push(#each_ident_name);
                        self
                    }
                ));
            }
        }
        token_stream.extend(quote::quote!(
            fn #field_name(&mut self,#field_name:#type_name)->&mut Self{
                #seeter_body;
                self
            }
        ));
    }
    let token_stream = quote::quote!(
        impl #new_struct_ident{
            #token_stream
        }
    );
    Ok(token_stream)
}

fn get_inner_type_of_option<'a>(ty: &'a syn::Type, types: &str) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        if let Some(op) = segments.last() {
            if &op.ident.to_string() == types {
                if let syn::PathSegment {
                    arguments:
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            args,
                            ..
                        }),
                    ..
                } = op
                {
                    if let Some(syn::GenericArgument::Type(t)) = args.first() {
                        return Some(t);
                    }
                }
            }
        }
    }

    None
}

fn impl_build_for_new_struct(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let new_struct_name = format!("{}Builder", st.ident.to_string());
    let new_struct_ident = syn::Ident::new(&new_struct_name, st.span());
    let old_struct_ident = &st.ident;
    let fields = get_struct_fields(st)?;
    let names: Vec<_> = fields.iter().map(|item| &item.ident).collect();
    let types: Vec<_> = fields.iter().map(|item| &item.ty).collect();
    let mut check_token_stream = proc_macro2::TokenStream::new();
    let mut propoty_token_stream = proc_macro2::TokenStream::new();
    for (ident, ty) in names.iter().zip(types.iter()) {
        if let Some(_t) = get_inner_type_of_option(ty, "Vec") {
            if let Some(ident) = ident {
                propoty_token_stream.extend(quote::quote!(
                    #ident:self.#ident.clone(),
                ));
            }
        } else {
            if let Some(_t) = get_inner_type_of_option(ty, "Option") {
                propoty_token_stream.extend(quote::quote!(
                    #ident:self.#ident.clone(),
                ));
            } else {
                propoty_token_stream.extend(quote::quote!(
                    #ident:self.#ident.as_ref().unwrap().clone(),
                ));
            }
        }

        if let Some(ident) = ident {
            if let Some(_) = get_inner_type_of_option(ty, "Vec") {
            } else {
                if let Some(_t) = get_inner_type_of_option(ty, "Option") {
                } else {
                    check_token_stream.extend(quote::quote!(
                        if self.#ident.is_none(){
                            let err_msg = format!("{} is need",stringify!(#ident));
                            return std::result::Result::Err(err_msg.into());
                        }
                    ));
                }
            }
        }
    }
    let res = quote::quote!(
        impl #new_struct_ident{
            pub fn build(&self)->std::result::Result<#old_struct_ident,std::boxed::Box<dyn std::error::Error>>{
                #check_token_stream
                let instance = #old_struct_ident{
                    #propoty_token_stream
                };
                std::result::Result::Ok(instance)
            }
        }
    );
    Ok(res)
}

fn get_attributes(field: &syn::Field) -> syn::Result<Option<syn::Ident>> {
    let attribute = field.attrs.first();
    if let Some(at) = attribute {
        let meta = at.parse_meta()?;
        if let syn::Meta::List(meta_list) = meta {
            let syn::MetaList {
                ref path,
                ref nested,
                ..
            } = meta_list;

            if let Some(seg) = path.segments.first() {
                if &seg.ident.to_string() == "builder" {
                    if let Some(syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue {
                        path: syn::Path { segments, .. },
                        lit,
                        ..
                    }))) = nested.first()
                    {
                        if let Some(ps) = segments.first() {
                            if ps.ident.to_string() == "each" {
                                if let syn::Lit::Str(lit_str) = lit {
                                    return Ok(Some(syn::Ident::new(
                                        lit_str.value().as_str(),
                                        field.span(),
                                    )));
                                }
                            } else {
                                return syn::Result::Err(syn::Error::new_spanned(
                                    meta_list,
                                    r#"expected `builder(each = "...")`"#,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}
