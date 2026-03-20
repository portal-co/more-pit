//! # pit-lang-generic
//!
//! Unified code-generator backend for PIT (Portal Interface Types) interfaces targeting
//! **C**, **Go**, **Haxe**, **TypeScript**, and **Swift**.
//!
//! All backends are unified under a single [`Opts<S>`] type parameterised by a [`Syntax`]
//! implementation.  The [`C`] syntax re-uses the [`c`] sub-module's `Display`-based
//! rendering pipeline so no intermediate [`String`] allocations occur during C output.
//!
//! ## Quick start
//!
//! ```ignore
//! use pit_lang_generic::{Opts, C, Go, Haxe, TypeScript, TypeScriptAsync, Swift};
//! use pit_core::Interface;
//!
//! let iface: Interface = /* … */;
//!
//! // C (Display-based, no intermediate String allocations)
//! let code = Opts::<C>::default().interface(&iface);
//! println!("{code}");
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
//! The C prefix is set on [`C::prefix`].
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

use alloc::{collections::btree_map::BTreeMap, format, string::{String, ToString}, vec::Vec};
use core::fmt::Display;
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
///
/// Return types are `impl Display` rather than `String` so that implementations
/// can avoid intermediate heap allocations (as [`C`] does via the [`c`] module's
/// wrapper types).  Implementations that build strings internally simply return
/// the `String` directly since `String: Display`.
pub trait Syntax: Default + Clone + core::fmt::Debug {
    /// Render a single argument type as a [`Display`] value.
    ///
    /// * `opts`    – the enclosing [`Opts`], giving access to `rewrites`
    /// * `arg`     – the PIT argument to render
    /// * `this`    – resource ID of the enclosing interface (needed for `ResTy::This`)
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a;

    /// Render a complete method signature as a [`Display`] value.
    ///
    /// The default implementation calls [`Self::render_ty`] for every parameter
    /// and return type; override only when the overall signature shape differs.
    ///
    /// * `opts` – the enclosing [`Opts`]
    /// * `sig`  – the PIT method signature
    /// * `this` – resource ID of the enclosing interface
    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a;

    /// Render a complete interface declaration as a [`Display`] value.
    ///
    /// * `opts`  – the enclosing [`Opts`]
    /// * `iface` – the PIT interface
    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a;
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared Opts struct
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for generating code in language `S`.
///
/// `S` implements [`Syntax`] and encodes all language-specific rendering rules.
/// `rewrites` applies to languages whose resource-type references span packages
/// (Go and Haxe); it is ignored by C / Swift / TypeScript which have no package system.
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
///
/// // C with a custom prefix
/// let mut opts = Opts::<C>::default();
/// opts.syntax.prefix = "my_".into();
/// ```
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub struct Opts<S: Syntax> {
    /// Maps 32-byte resource IDs to target-language package/module import paths.
    ///
    /// Used by [`Go`] and [`Haxe`].  Other backends ignore this field.
    pub rewrites: BTreeMap<[u8; 32], String>,

    /// The [`Syntax`] instance.  Zero-sized for most backends; holds [`C::prefix`]
    /// for the C backend.
    pub syntax: S,
}

impl<S: Syntax> Opts<S> {
    /// Convert a PIT argument type to its target-language representation.
    ///
    /// Returns `impl Display`; call `.to_string()` only when a heap-allocated
    /// [`String`] is strictly required.
    ///
    /// Delegates to [`Syntax::render_ty`].
    #[inline]
    pub fn ty<'a>(&'a self, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
        S::render_ty(self, arg, this)
    }

    /// Convert a PIT method signature to its target-language representation.
    ///
    /// Returns `impl Display`; call `.to_string()` only when a heap-allocated
    /// [`String`] is strictly required.
    ///
    /// Delegates to [`Syntax::render_meth`].
    #[inline]
    pub fn meth<'a>(&'a self, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a {
        S::render_meth(self, sig, this)
    }

    /// Convert a PIT interface to a complete target-language declaration.
    ///
    /// Returns `impl Display`; call `.to_string()` only when a heap-allocated
    /// [`String`] is strictly required.
    ///
    /// Delegates to [`Syntax::render_interface`].
    #[inline]
    pub fn interface<'a>(&'a self, iface: &'a Interface) -> impl Display + 'a {
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
// C syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for C.
///
/// Delegates to the [`c`] module's `Display`-based pipeline, so no intermediate
/// [`String`] allocations occur for type / method / interface rendering.
///
/// Generated code uses C preprocessor macros:
/// ```c
/// #define <prefix><hex_id>_t_IFACE_method(CUR,METH)  vfunc(struct{…},METH,VSelf,…)
/// #define <prefix><hex_id>_t_IFACE  …
/// interface(<prefix><hex_id>_t)
/// ```
///
/// Set [`C::prefix`] (via [`Opts::syntax`]) to customise the type-name prefix:
/// ```ignore
/// let mut opts = Opts::<C>::default();
/// opts.syntax.prefix = "my_".into();
/// println!("{}", opts.interface(&iface));
/// ```
#[derive(Default, Clone, Debug)]
pub struct C {
    /// Prefix prepended to every generated C type name (maps to [`c::PureC::cx`]).
    pub prefix: String,
}

impl Syntax for C {
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, _this: [u8; 32]) -> impl Display + 'a {
        c::C {
            value: arg,
            kind: c::PureC {
                cx: opts.syntax.prefix.as_str(),
            },
        }
    }

    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, _this: [u8; 32]) -> impl Display + 'a {
        c::C {
            value: sig,
            kind: c::PureC {
                cx: opts.syntax.prefix.as_str(),
            },
        }
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
        c::C {
            value: iface,
            kind: c::PureC {
                cx: opts.syntax.prefix.as_str(),
            },
        }
    }
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
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
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

    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a {
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
            .map(|a| opts.ty(a, this).to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("({params}) ({rets})")
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
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
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
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

    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a {
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

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
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
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
        TypeScript::ty_inner(opts, arg, this, "")
    }

    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a {
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
            .map(|a| opts.ty(a, this).to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("({params}): [{rets}]")
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
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
    fn render_ty<'a>(_opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
        // Delegate to the shared helper with the async "A" prefix.
        // We build a temporary sync Opts with no rewrites — rewrites are not used
        // by TypeScript's ty_inner, so this is always correct.
        let tmp = Opts::<TypeScript>::default();
        TypeScript::ty_inner(&tmp, arg, this, "A")
    }

    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a {
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
            .map(|a| opts.ty(a, this).to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("({params}): [{rets}]| Promise<[{rets}]>")
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
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
    fn render_ty<'a>(_opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
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

    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a {
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

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
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

/// Type alias for C code generation options.
pub type COpts      = Opts<C>;
/// Type alias for Go code generation options.
pub type GoOpts     = Opts<Go>;
/// Type alias for Haxe code generation options.
pub type HaxeOpts   = Opts<Haxe>;
/// Type alias for TypeScript (sync) code generation options.
pub type TsOpts     = Opts<TypeScript>;
/// Type alias for TypeScript (async) code generation options.
pub type TsOptsAsync = Opts<TypeScriptAsync>;
/// Type alias for Swift code generation options.
pub type SwiftOpts  = Opts<Swift>;
