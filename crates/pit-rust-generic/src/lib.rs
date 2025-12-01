//! # pit-rust-generic
//!
//! Code generator for Rust trait definitions from PIT (Portal Interface Types) interfaces.
//!
//! This crate generates Rust traits from PIT interface definitions using `proc-macro2` and `quote`.
//! The generated traits can be used to implement cross-language interface boundaries in Rust.
//!
//! ## Overview
//!
//! The main entry points are:
//! - [`interface`] - Generates a complete Rust trait from a PIT [`Interface`]
//! - [`sig`] - Generates a method signature from a PIT [`Sig`]
//! - [`arg`] - Generates a type expression from a PIT [`Arg`]
//!
//! ## Example
//!
//! ```ignore
//! use pit_rust_generic::{Params, interface};
//! use pit_core::Interface;
//!
//! let iface: Interface = /* parse from PIT format */;
//! let params = Params {
//!     core: syn::parse_quote!(::core),
//!     flags: Default::default(),
//!     asyncness: None,
//! };
//! let tokens = interface(&params, &iface);
//! ```
//!
//! ## Features
//!
//! - `unstable-sdk` - Enable portal-solutions-sdk integration
//! - `unstable-pcode` - Enable pcode expression support
//! - `unstable-sdkcode` - Combined SDK and pcode support
//! - `unstable-generics` - Enable generic parameter support

use pit_core::{Arg, Interface, Sig};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::token::Async;

/// Configuration parameters for code generation.
///
/// Controls how Rust code is generated from PIT interfaces.
pub struct Params {
    /// Path to the core library (e.g., `::core` or `::std`).
    /// Used as the prefix for standard library types in generated code.
    pub core: syn::Path,
    /// Feature flags controlling code generation behavior.
    pub flags: FeatureFlags,
    /// If `Some`, generates async trait methods.
    pub asyncness: Option<Async>,
}

/// Feature flags controlling advanced code generation options.
///
/// These flags enable unstable or experimental features in the generated code.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
#[non_exhaustive]
pub struct FeatureFlags {
    /// Enable Rust's specialization feature.
    ///
    /// When enabled, generates default trait implementations that can be specialized.
    /// Requires `#![feature(specialization)]` in the consuming crate.
    pub specialization: bool,
}

/// Generates a Rust type expression from a PIT argument type.
///
/// Converts a [`pit_core::Arg`] into a [`TokenStream`] representing the corresponding
/// Rust type. Handles primitive types, resources, and their modifiers.
///
/// # Arguments
///
/// * `p` - Code generation parameters
/// * `a` - The PIT argument type to convert
/// * `root` - The 32-byte resource ID of the containing interface (for `this` references)
///
/// # Returns
///
/// A `TokenStream` containing the Rust type expression.
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

/// Generates a Rust method signature from a PIT method signature.
///
/// Converts a [`pit_core::Sig`] into a [`TokenStream`] representing a Rust method
/// signature with parameters and return type.
///
/// # Arguments
///
/// * `p` - Code generation parameters
/// * `s` - The PIT method signature to convert
/// * `root` - The 32-byte resource ID of the containing interface
///
/// # Returns
///
/// A `TokenStream` containing the method signature (parameters and return type).
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

/// Generates a complete Rust trait definition from a PIT interface.
///
/// This is the main entry point for generating Rust code from PIT interfaces.
/// It produces a trait definition along with an associated error type.
///
/// # Generated Code Structure
///
/// The generated code includes:
/// - An error struct named `P<interface_id>Error`
/// - A trait named `P<interface_id>` with:
///   - An associated `Error` type
///   - Methods corresponding to the interface methods
/// - Optionally, a default implementation using specialization
///
/// # Arguments
///
/// * `p` - Code generation parameters
/// * `i` - The PIT interface to convert
///
/// # Returns
///
/// A `TokenStream` containing the complete trait definition and supporting types.
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
                    default impl<'bound,T: #core::marker::Sized + 'bound> #x<'bound> for T{
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
        const _: () = {
            impl #core::fmt::Display for #xe{
                fn fmt(&self, a: &mut #core::fmt::Formatter) -> #core::fmt::Result{
                    #core::fmt::Result::Ok(())
                }
            }
            impl #core::error::Error for #xe{}
        };
        pub trait #x<'bound>: 'bound{
            type Error: #core::error::Error;
            #(#methods);*
        }
        #spec
    }
}
