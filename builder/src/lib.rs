use proc_macro::TokenStream;
use proc_macro2;
use syn::{spanned::Spanned};
use quote;

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let st = syn::parse_macro_input!(input as syn::DeriveInput);
    match build(st){
        Ok(token_stream) => token_stream.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn build(st:syn::DeriveInput)->syn::Result<proc_macro2::TokenStream>{
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

type StructFields = syn::punctuated::Punctuated<syn::Field,syn::Token![,]>;

fn get_struct_fields(st:&syn::DeriveInput)->syn::Result<&StructFields>{
    if let syn::Data::Struct(
        syn::DataStruct{
            fields:syn::Fields::Named(
                syn::FieldsNamed{
                    named,
                    ..
                }
            ),
            ..
        }
    ) = &st.data{
        return Ok(named)
    }else{
        Err(syn::Error::new_spanned(&st.ident,"need a struct,but found a enum"))
    }
}

fn build_new_struct(st:&syn::DeriveInput)->syn::Result<proc_macro2::TokenStream>{
    let struct_name = st.ident.to_string();
    let new_struct_name = format!("{}Builder",struct_name);
    let new_struct_ident = syn::Ident::new(&new_struct_name,st.span());
    let struct_vis = &st.vis;
    let fields = get_struct_fields(st)?;
    let names:Vec<_> = fields.iter().map(|item|&item.ident).collect();
    let types:Vec<_> = fields.iter().map(|item|{
        match get_inner_type_of_option(&item.ty){
            Some(t) => t,
            None => &item.ty,
        }
    }).collect();
    let vis:Vec<_> = fields.iter().map(|item|&item.vis).collect();

    let ret = quote::quote!(
        #struct_vis struct #new_struct_ident{
            #(#vis #names:std::option::Option<#types>,)*
        }
    );
    Ok(ret)
}

fn impl_for_old_struct(st:&syn::DeriveInput)->syn::Result<proc_macro2::TokenStream>{
    let old_struct_ident = &st.ident;
    let new_struct_name = format!("{}Builder",old_struct_ident.to_string());
    let new_struct_ident = syn::Ident::new(&new_struct_name,st.span());

    let fields = get_struct_fields(st)?;
    let names:Vec<_> = fields.iter().map(|item|&item.ident).collect();

    let ret = quote::quote!(
        impl #old_struct_ident{
            pub fn builder()->#new_struct_ident{
                #new_struct_ident{
                    #(#names:std::option::Option::None,)*
                }
            }
        }
    );
    Ok(ret)
}

fn impl_for_new_struct_setter(st:&syn::DeriveInput)->syn::Result<proc_macro2::TokenStream>{
    let new_struct_name = format!("{}Builder",st.ident.to_string());
    let new_struct_ident = syn::Ident::new(&new_struct_name,st.span());

    let fields = get_struct_fields(st)?;
    let names:Vec<_> = fields.iter().map(|item|&item.ident).collect();
    let types:Vec<_> = fields.iter().map(|item|{
        match get_inner_type_of_option(&item.ty){
            Some(t) => t,
            None => &item.ty,
        }
    }).collect();
    let token_stream = quote::quote!(
        impl #new_struct_ident{
            #(
                fn #names(&mut self,#names:#types)->&mut Self{
                    self.#names = std::option::Option::Some(#names);
                    self
                }
            )*
        }
        
    );
    Ok(token_stream)
}

fn get_inner_type_of_option(ty:&syn::Type)->Option<&syn::Type>{
    if let syn::Type::Path(syn::TypePath{
        path:syn::Path{
            segments,
            ..
        },
        ..
    }) = ty{
        if let Some(op) = segments.last(){
            if &op.ident.to_string() == "Option"{
                if let syn::PathSegment{
                    arguments:syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments{
                        args,
                        ..
                    }),
                    ..
                } = op{
                    if let Some(syn::GenericArgument::Type(t)) = args.first(){
                        return Some(t);
                    }
                }
            }
        }
    }

    None
}

fn impl_build_for_new_struct(st:&syn::DeriveInput)->syn::Result<proc_macro2::TokenStream>{
    let new_struct_name = format!("{}Builder",st.ident.to_string());
    let new_struct_ident = syn::Ident::new(&new_struct_name,st.span());
    let old_struct_ident = &st.ident;
    let fields = get_struct_fields(st)?;
    let names:Vec<_> = fields.iter().map(|item|&item.ident).collect();
    let types:Vec<_> = fields.iter().map(|item|&item.ty).collect();
    let mut check_token_stream = proc_macro2::TokenStream::new();
    let mut propoty_token_stream = proc_macro2::TokenStream::new();
    for (ident,ty) in names.iter().zip(types.iter()){
        if let Some(_t) = get_inner_type_of_option(ty){
            if let Some(ident) = ident{
                propoty_token_stream.extend(quote::quote!(
                    #ident:self.#ident.clone(),
                ));
            }
            continue;
        }
        if let Some(ident) = ident{
            check_token_stream.extend(quote::quote!(
                if self.#ident.is_none(){
                    let err_msg = format!("{} is need",stringify!(#ident));
                    return std::result::Result::Err(err_msg.into());
                }
            ));
            propoty_token_stream.extend(quote::quote!(
                #ident:self.#ident.clone().unwrap(),
            ));
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