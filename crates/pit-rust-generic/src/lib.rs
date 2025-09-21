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
pub struct FeatureFlags {
    pub specialization: bool,
}
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
                        impl #core::marker::Sized + 'bound
                    };
                }
                pit_core::ResTy::Of(a) => *a,
                pit_core::ResTy::This => root,
                _ => {
                    if p.flags.specialization {
                        return quote! {
                            impl #core::marker::Sized + 'bound
                        };
                    } else {
                        return quote! {
                            #core::convert::Infallible
                        };
                    }
                }
            };
            let x = hex::encode(&x);
            let x = format_ident!(
                "P{}{}{x}",
                match p.flags.specialization {
                    false => "",
                    true => "S",
                },
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
        _ => {
            if p.flags.specialization {
                return quote! {
                    impl #core::marker::Sized + 'bound
                };
            } else {
                return quote! {
                    #core::convert::Infallible
                };
            }
        }
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
        "P{}{}{x}",
        match p.flags.specialization {
            false => "",
            true => "S",
        },
        match asyncness.as_ref() {
            None => "",
            Some(_) => "async",
        }
    );
    let xe = format_ident!("{x}Error");
    let core = &p.core;

    let methods = i
        .methods
        .iter()
        .map(|(a, b)| {
            let asyncness = asyncness.iter();
            let a = format_ident!("{a}");
            let b = sig(p, b, root);
            quote! {
                #(#asyncness)* fn  #a #b
            }
        })
        .collect::<Vec<_>>();
    let spec = match p.flags.specialization {
        false => quote! {},
        true => {
            let method_impls = methods.iter().map(|a| {
                quote! {
                    #a {
                        return #core::result::Result::Err(#xe{})
                    }
                }
            });
            quote! {
                const _: () = {
                    default impl<'bound,T: #core::marker::Sized> #x<'bound> for T{
                        type Error = #xe;
                        #(#method_impls);*
                    }
                }
            }
        }
    };
    quote! {
        #[derive(#core::Clone,#core::Copy,#core::Debug)]
        struct #xe{}
        impl #core::fmt::Display for #xe{
            fn fmt(&self, a: &mut #core::fmt::Formatter) -> #core::fmt::Result{
                #core::fmt::Result::Ok(())
            }
        }
        pub trait #x<'bound>: 'bound{
            type Error: #core::error::Error;
            #(#methods);*
        }
        #spec
    }
}
