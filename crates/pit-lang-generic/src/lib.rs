//! # pit-lang-generic
//!
//! Unified code-generator backend for PIT (Portal Interface Types) interfaces targeting
//! **C**, **Go**, **Haxe**, **TypeScript**, and **Swift**.
//!
//! All four OOP-style backends (Go, Haxe, TypeScript, Swift) are unified under a single
//! [`Opts<S>`] type parameterised by a [`Syntax`] implementation.  The C backend keeps its
//! own [`c`] sub-module because it uses a structurally different `Display`-based rendering
//! pipeline.
//!
//! ## Quick start
//!
//! ```ignore
//! use pit_lang_generic::{Opts, Go, Haxe, TypeScript, TypeScriptAsync, Swift};
//! use pit_core::Interface;
//!
//! let iface: Interface = /* … */;
//!
//! // Go
//! let code = Opts::<Go>::default().interface(&iface);
//!
//! // Haxe
//! let code = Opts::<Haxe>::default().interface(&iface);
//!
//! // TypeScript (sync)
//! let code = Opts::<TypeScript>::default().interface(&iface);
//!
//! // TypeScript (async / Promise)
//! let code = Opts::<TypeScriptAsync>::default().interface(&iface);
//!
//! // Swift
//! let code = Opts::<Swift>::default().interface(&iface);
//! ```
//!
//! Package rewrites (Go / Haxe) are set on [`Opts::rewrites`].
//!
//! ## C backend
//!
//! ```ignore
//! use pit_lang_generic::c::{C, PureC};
//! use pit_core::Interface;
//!
//! let iface: Interface = /* … */;
//! println!("{}", C { value: &iface, kind: PureC { cx: "my_" } });
//! ```
//!
//! ## Feature flags
//!
//! | Flag | Effect |
//! |------|--------|
//! | `unstable-sdk` | Enable portal-solutions-sdk integration |
//! | `unstable-pcode` | Enable pcode expression support |
//! | `unstable-sdkcode` | Combined SDK + pcode |
//! | `unstable-generics` | Generic parameter support |

#![no_std]
extern crate alloc;

use alloc::{collections::btree_map::BTreeMap, format, string::String, vec::Vec};
use pit_core::{Arg, Interface, ResTy, Sig};

// ─────────────────────────────────────────────────────────────────────────────
// C backend (Display-based; kept in its own module)
// ─────────────────────────────────────────────────────────────────────────────

pub mod c;

// ─────────────────────────────────────────────────────────────────────────────
// Syntax trait
// ─────────────────────────────────────────────────────────────────────────────

/// Defines the language-specific rendering rules used by [`Opts`].
///
/// Implement this trait to add support for a new target language.
/// Three methods map the three levels of a PIT interface hierarchy:
/// argument type → method signature → full interface declaration.
pub trait Syntax: Default + Clone + core::fmt::Debug {
    /// Render a single argument type to a [`String`].
    ///
    /// * `opts`    – the enclosing [`Opts`], giving access to `rewrites`
    /// * `arg`     – the PIT argument to render
    /// * `this`    – resource ID of the enclosing interface (needed for `ResTy::This`)
    fn render_ty(opts: &Opts<Self>, arg: &Arg, this: [u8; 32]) -> String;

    /// Render a complete method signature to a [`String`].
    ///
    /// The default implementation calls [`Self::render_ty`] for every parameter
    /// and return type; override only when the overall signature shape differs.
    ///
    /// * `opts` – the enclosing [`Opts`]
    /// * `sig`  – the PIT method signature
    /// * `this` – resource ID of the enclosing interface
    fn render_meth(opts: &Opts<Self>, sig: &Sig, this: [u8; 32]) -> String;

    /// Render a complete interface declaration to a [`String`].
    ///
    /// * `opts`  – the enclosing [`Opts`]
    /// * `iface` – the PIT interface
    fn render_interface(opts: &Opts<Self>, iface: &Interface) -> String;
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared Opts struct
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for generating code in language `S`.
///
/// `S` implements [`Syntax`] and encodes all language-specific rendering rules.
/// `rewrites` applies to languages whose resource-type references span packages
/// (Go and Haxe); it is ignored by Swift / TypeScript which have no package system.
///
/// # Constructing
///
/// ```ignore
/// // Zero rewrites — just use Default
/// let opts = Opts::<Go>::default();
///
/// // With rewrites
/// let mut opts = Opts::<Go>::default();
/// opts.rewrites.insert(some_rid, "mypkg".into());
/// ```
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub struct Opts<S: Syntax> {
    /// Maps 32-byte resource IDs to target-language package/module import paths.
    ///
    /// Used by [`Go`] and [`Haxe`].  Other backends ignore this field.
    pub rewrites: BTreeMap<[u8; 32], String>,

    /// The [`Syntax`] instance.  Because all current syntax types are zero-sized,
    /// this is a `PhantomData`-equivalent that carries `S`'s impl at zero cost.
    pub syntax: S,
}

impl<S: Syntax> Opts<S> {
    /// Convert a PIT argument type to its target-language string.
    ///
    /// Delegates to [`Syntax::render_ty`].
    #[inline]
    pub fn ty(&self, arg: &Arg, this: [u8; 32]) -> String {
        S::render_ty(self, arg, this)
    }

    /// Convert a PIT method signature to its target-language string.
    ///
    /// Delegates to [`Syntax::render_meth`].
    #[inline]
    pub fn meth(&self, sig: &Sig, this: [u8; 32]) -> String {
        S::render_meth(self, sig, this)
    }

    /// Convert a PIT interface to a complete target-language declaration string.
    ///
    /// Delegates to [`Syntax::render_interface`].
    #[inline]
    pub fn interface(&self, iface: &Interface) -> String {
        S::render_interface(self, iface)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: rewrite a resource-ID to the correct package-qualified name
// ─────────────────────────────────────────────────────────────────────────────

/// Build a package-qualified name for a resource `id` using `opts.rewrites`.
///
/// Returns `"<pkg>.<name>"` when a rewrite exists, otherwise
/// `"<default_pkg_prefix><hex_id>.<name>"`.
fn rewrite_pkg<S: Syntax>(
    opts: &Opts<S>,
    id: &[u8; 32],
    default_pkg_prefix: &str,
    name: &str,
) -> String {
    let pkg = match opts.rewrites.get(id) {
        Some(p) => p.clone(),
        None => format!("{}{}", default_pkg_prefix, hex::encode(id)),
    };
    format!("{}.{}", pkg, name)
}

// ─────────────────────────────────────────────────────────────────────────────
// Go syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for Go.
///
/// Generated interfaces look like:
/// ```go
/// type P<hex_id> interface{P<hex_id>_method (p0 uint32) (uint64)}
/// ```
#[derive(Default, Clone, Debug)]
pub struct Go;

impl Syntax for Go {
    fn render_ty(opts: &Opts<Self>, arg: &Arg, this: [u8; 32]) -> String {
        match arg {
            Arg::I32 => format!("uint32"),
            Arg::I64 => format!("uint64"),
            Arg::F32 => format!("float32"),
            Arg::F64 => format!("float64"),
            Arg::Resource { ty, .. } => match ty {
                ResTy::None => format!("interface{{}}"),
                ResTy::Of(id) => rewrite_pkg(opts, id, "pit", &format!("P{}", hex::encode(id))),
                ResTy::This => format!("P{}", hex::encode(this)),
                _ => todo!(),
            },
            _ => todo!(),
        }
    }

    fn render_meth(opts: &Opts<Self>, sig: &Sig, this: [u8; 32]) -> String {
        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, a)| format!("p{i} {}", opts.ty(a, this)))
            .collect::<Vec<_>>()
            .join(",");
        let rets = sig
            .rets
            .iter()
            .map(|a| opts.ty(a, this))
            .collect::<Vec<_>>()
            .join(",");
        format!("({params}) ({rets})")
    }

    fn render_interface(opts: &Opts<Self>, iface: &Interface) -> String {
        let this = iface.rid();
        let hex = hex::encode(this);
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("P{hex}_{name} {}", opts.meth(sig, this)))
            .collect::<Vec<_>>()
            .join("");
        format!("type P{hex} interface{{{methods}}}")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Haxe syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for Haxe.
///
/// Generated interfaces look like:
/// ```haxe
/// interface P<hex_id> {P<hex_id>_method (p0: haxe.Int32): {r0: Float}}
/// ```
#[derive(Default, Clone, Debug)]
pub struct Haxe;

impl Syntax for Haxe {
    fn render_ty(opts: &Opts<Self>, arg: &Arg, this: [u8; 32]) -> String {
        match arg {
            Arg::I32 => format!("haxe.Int32"),
            Arg::I64 => format!("haxe.Int32"),
            Arg::F32 => format!("Float"),
            Arg::F64 => format!("Float"),
            Arg::Resource { ty, .. } => match ty {
                ResTy::None => format!("Dynamic"),
                ResTy::Of(id) => rewrite_pkg(opts, id, "pit", &format!("P{}", hex::encode(id))),
                ResTy::This => format!("P{}", hex::encode(this)),
                _ => todo!(),
            },
            _ => todo!(),
        }
    }

    fn render_meth(opts: &Opts<Self>, sig: &Sig, this: [u8; 32]) -> String {
        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, a)| format!("p{i}: {}", opts.ty(a, this)))
            .collect::<Vec<_>>()
            .join(",");
        let rets = sig
            .rets
            .iter()
            .enumerate()
            .map(|(i, a)| format!("r{i}: {}", opts.ty(a, this)))
            .collect::<Vec<_>>()
            .join(",");
        format!("({params}): {{{rets}}}")
    }

    fn render_interface(opts: &Opts<Self>, iface: &Interface) -> String {
        let this = iface.rid();
        let hex = hex::encode(this);
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("P{hex}_{name} {}", opts.meth(sig, this)))
            .collect::<Vec<_>>()
            .join("");
        format!("interface P{hex} {{{methods}}}")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TypeScript (sync) syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for TypeScript (synchronous).
///
/// Generated types look like:
/// ```typescript
/// export type P<hex_id> = {P<hex_id>_method (p0: number): [number]}
/// ```
#[derive(Default, Clone, Debug)]
pub struct TypeScript;

impl TypeScript {
    fn ty_inner(_opts: &Opts<Self>, arg: &Arg, this: [u8; 32], prefix: &str) -> String {
        match arg {
            Arg::I32 => format!("number"),
            Arg::I64 => format!("bigint"),
            Arg::F32 => format!("number"),
            Arg::F64 => format!("number"),
            Arg::Resource {
                ty, nullable, ..
            } => {
                let base = match ty {
                    ResTy::None => format!("any"),
                    ResTy::Of(id) => format!("{prefix}P{}", hex::encode(id)),
                    ResTy::This => format!("{prefix}P{}", hex::encode(this)),
                    _ => todo!(),
                };
                if *nullable {
                    format!("{base} | undefined")
                } else {
                    base
                }
            }
            _ => todo!(),
        }
    }
}

impl Syntax for TypeScript {
    fn render_ty(opts: &Opts<Self>, arg: &Arg, this: [u8; 32]) -> String {
        TypeScript::ty_inner(opts, arg, this, "")
    }

    fn render_meth(opts: &Opts<Self>, sig: &Sig, this: [u8; 32]) -> String {
        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, a)| format!("p{i}: {}", opts.ty(a, this)))
            .collect::<Vec<_>>()
            .join(",");
        let rets = sig
            .rets
            .iter()
            .map(|a| opts.ty(a, this))
            .collect::<Vec<_>>()
            .join(",");
        format!("({params}): [{rets}]")
    }

    fn render_interface(opts: &Opts<Self>, iface: &Interface) -> String {
        let this = iface.rid();
        let hex = hex::encode(this);
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("P{hex}_{name} {}", opts.meth(sig, this)))
            .collect::<Vec<_>>()
            .join("");
        format!("export type P{hex} = {{{methods}}}")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TypeScript (async) syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for TypeScript with async / Promise support.
///
/// Type names are prefixed with `A` (e.g. `AP<hex_id>`).
/// Return types include a `| Promise<[…]>` variant.
///
/// Generated types look like:
/// ```typescript
/// export type AP<hex_id> = {AP<hex_id>_method (p0: number): [number]| Promise<[number]>}
/// ```
#[derive(Default, Clone, Debug)]
pub struct TypeScriptAsync;

impl Syntax for TypeScriptAsync {
    fn render_ty(_opts: &Opts<Self>, arg: &Arg, this: [u8; 32]) -> String {
        // Delegate to the shared helper with the async "A" prefix.
        // We build a temporary sync Opts with no rewrites — rewrites are not used
        // by TypeScript's ty_inner, so this is always correct.
        let tmp = Opts::<TypeScript>::default();
        TypeScript::ty_inner(&tmp, arg, this, "A")
    }

    fn render_meth(opts: &Opts<Self>, sig: &Sig, this: [u8; 32]) -> String {
        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, a)| format!("p{i}: {}", opts.ty(a, this)))
            .collect::<Vec<_>>()
            .join(",");
        let rets = sig
            .rets
            .iter()
            .map(|a| opts.ty(a, this))
            .collect::<Vec<_>>()
            .join(",");
        format!("({params}): [{rets}]| Promise<[{rets}]>")
    }

    fn render_interface(opts: &Opts<Self>, iface: &Interface) -> String {
        let this = iface.rid();
        let hex = hex::encode(this);
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("AP{hex}_{name} {}", opts.meth(sig, this)))
            .collect::<Vec<_>>()
            .join("");
        format!("export type AP{hex} = {{{methods}}}")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Swift syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for Swift.
///
/// Generated protocols look like:
/// ```swift
/// open protocol P<hex_id> {open P<hex_id>_method (p0 _: UInt32) -> (r0 _: UInt64)}
/// ```
#[derive(Default, Clone, Debug)]
pub struct Swift;

impl Syntax for Swift {
    fn render_ty(_opts: &Opts<Self>, arg: &Arg, this: [u8; 32]) -> String {
        match arg {
            Arg::I32 => format!("UInt32"),
            Arg::I64 => format!("UInt64"),
            Arg::F32 => format!("Float"),
            Arg::F64 => format!("Double"),
            Arg::Resource { ty, .. } => match ty {
                ResTy::None => format!("Any"),
                ResTy::Of(id) => format!("any P{}", hex::encode(id)),
                ResTy::This => format!("any P{}", hex::encode(this)),
                _ => todo!(),
            },
            _ => todo!(),
        }
    }

    fn render_meth(opts: &Opts<Self>, sig: &Sig, this: [u8; 32]) -> String {
        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, a)| format!("p{i} _: {}", opts.ty(a, this)))
            .collect::<Vec<_>>()
            .join(",");
        let rets = sig
            .rets
            .iter()
            .enumerate()
            .map(|(i, a)| format!("r{i} _: {}", opts.ty(a, this)))
            .collect::<Vec<_>>()
            .join(",");
        format!("({params}) -> ({rets})")
    }

    fn render_interface(opts: &Opts<Self>, iface: &Interface) -> String {
        let this = iface.rid();
        let hex = hex::encode(this);
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("open P{hex}_{name} {}", opts.meth(sig, this)))
            .collect::<Vec<_>>()
            .join("");
        format!("open protocol P{hex} {{{methods}}}")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience type aliases matching the old per-crate public API
// ─────────────────────────────────────────────────────────────────────────────

/// Type alias for Go code generation options.
pub type GoOpts      = Opts<Go>;
/// Type alias for Haxe code generation options.
pub type HaxeOpts    = Opts<Haxe>;
/// Type alias for TypeScript (sync) code generation options.
pub type TsOpts      = Opts<TypeScript>;
/// Type alias for TypeScript (async) code generation options.
pub type TsOptsAsync = Opts<TypeScriptAsync>;
/// Type alias for Swift code generation options.
pub type SwiftOpts   = Opts<Swift>;
