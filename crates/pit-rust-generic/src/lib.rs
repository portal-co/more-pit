use pit_core::{Arg, Interface, Sig};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::token::Async;
pub struct Params {
    pub core: syn::Path,
    pub flags: FeatureFlags,
    pub asyncness: Option<Async>,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
#[non_exhaustive]
pub struct FeatureFlags {}
pub fn arg(p: &Params, a: &Arg, root: [u8; 32]) -> TokenStream {
    let core = &p.core;
    let asyncness = &p.asyncness;
    match a {
        Arg::I32 => quote! {#core::primitive::u32},
        Arg::I64 => quote! {#core::primitive::u64},
        Arg::F32 => quote! {#core::primitive::f32},
        Arg::F64 => quote! {#core::primitive::f64},
        Arg::Resource {
            ty,
            nullable,
            take,
            ann,
        } => {
            let x = match ty {
                pit_core::ResTy::None => {
                    return quote! {
                        impl #core::any::Any + 'bound
                    };
                }
                pit_core::ResTy::Of(a) => *a,
                pit_core::ResTy::This => root,
                _ => {
                    return quote! {
                        #core::convert::Infallible
                    };
                }
            };
            let x = hex::encode(&x);
            let x = format_ident!(
                "P{}{x}",
                match asyncness.as_ref() {
                    None => "",
                    Some(_) => "async",
                }
            );
            let mut a = quote! {
                impl #x<'bound,Error = Self::Error> + 'bound
            };
            if !*take {
                a = quote! {
                    impl #core::ops::DerefMut<Target = #a> + 'bound
                }
            }
            if *nullable {
                a = quote! {
                    #core::option::Option<#a>
                }
            }
            a
        }
        _ => quote! {
            #core::convert::Infallible
        },
    }
}
pub fn sig(p: &Params, s: &Sig, root: [u8; 32]) -> TokenStream {
    let params = s.params.iter().enumerate().map(|(a, b)| {
        let a = format_ident!("arg{a}");
        let b = arg(p, b, root);
        quote! {
            #a : #b
        }
    });
    let rets = s.rets.iter().map(|a| arg(p, a, root));
    let core = &p.core;
    quote! {
        (&mut self, #(#params),*) -> #core::result::Result<(#(#rets),*),Self::Error>
    }
}
pub fn interface(p: &Params, i: &Interface) -> TokenStream {
    let root = i.rid();
    let asyncness = &p.asyncness;
    let x = hex::encode(&root);
    let x = format_ident!(
        "P{}{x}",
        match asyncness.as_ref() {
            None => "",
            Some(_) => "async",
        }
    );
    let core = &p.core;

    let methods = i.methods.iter().map(|(a, b)| {
        let asyncness = asyncness.iter();
        let a = format_ident!("{a}");
        let b = sig(p, b, root);
        quote! {
            #(#asyncness)* fn  #a #b
        }
    });
    quote! {
        pub trait #x<'bound>: 'bound{
            type Error: #core::error::Error;
            #(#methods);*
        }
    }
}
