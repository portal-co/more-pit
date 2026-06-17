//! # pit-lang-generic
//!
//! Unified code-generator backend for PIT (Portal Interface Types) interfaces targeting
//! **C**, **Go**, **Haxe**, **TypeScript**, **Swift**, **Haskell**, and **Rust**
//! (Rust via [`pit-rust-generic`]).
//!
//! All non-Rust backends are unified under a single [`Opts<S>`] type parameterised
//! by a [`Syntax`] implementation.  The [`C`] syntax re-uses the [`c`] sub-module's
//! `Display`-based rendering pipeline so no intermediate [`String`] allocations occur
//! during C output.
//!
//! ## Quick start
//!
//! ```ignore
//! use pit_lang_generic::{Opts, C, Go, Haxe, TypeScript, TypeScriptAsync, Swift, Haskell};
//! use pit_core::Interface;
//!
//! let iface: Interface = /* … */;
//!
//! // C — interface99 header macros
//! let code = Opts::<C>::default().interface(&iface);
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
//!
//! // Haskell (monad-generic with associated resource type)
//! let code = Opts::<Haskell>::default().interface(&iface);
//!
//! // Complete source file (includes module header + imports):
//! let file: String = Opts::<Haskell>::default().file(&iface);
//! ```
//!
//! ## Module-per-file generation
//!
//! [`Syntax::render_file`] / [`Opts::file`] produce a complete source file
//! including any language-specific preamble and import statements for
//! dependencies.  Populate [`Opts::rewrites`] with the direct-dependency map
//! before calling `file()`:
//!
//! ```ignore
//! let mut opts = Opts::<Haskell>::default();
//! // dep_rid → module name (e.g. "P<dep_hex>")
//! opts.rewrites.insert(dep_rid, "P<dep_hex>".into());
//! let src: String = opts.file(&iface);
//! ```
//!
//! The `pit-gen` CLI constructs this map automatically from the full `.pit` file graph.
//!
//! ## Cross-language notes
//!
//! | Language | borrow flags | nullable | module system |
//! |---|---|---|---|
//! | C | ignored | `T *` pointer | `#include` per dep |
//! | Go | ignored | Go interfaces are already nilable | single package |
//! | Haxe | ignored | `Null<T>` | `package pit;` |
//! | TypeScript | ignored | `T \| undefined` | `import type` per dep |
//! | Swift | ignored | `T?` | all files compiled together |
//! | Haskell | ignored | `Maybe T` | `import qualified` per dep |
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

use alloc::{
    collections::btree_map::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Display;
use pit_core::{Arg, ArgTy, Interface, ResTy, Sig};

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
/// Four methods cover the full generation surface:
///
/// ```text
/// render_ty        – single argument type
/// render_meth      – method signature (params + return)
/// render_interface – complete type/interface/protocol/class declaration
/// render_file      – complete source file (preamble + imports + declaration)
/// ```
///
/// The three `render_*` methods return `impl Display` to avoid mandatory heap
/// allocation; implementations that build strings simply return `String` since
/// `String: Display`.
///
/// `render_file` returns `String` and has a sensible default that delegates to
/// `render_interface`, so existing implementations remain valid.
pub trait Syntax: Default + Clone + core::fmt::Debug {
    /// Render a single argument type as a [`Display`] value.
    ///
    /// * `opts` – the enclosing [`Opts`], providing `rewrites` and `syntax`
    /// * `arg`  – the PIT argument to render
    /// * `this` – resource ID of the enclosing interface (for `ResTy::This`)
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a;

    /// Render a complete method signature as a [`Display`] value.
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

    /// Render a complete source file for a single PIT interface.
    ///
    /// Includes any language-specific file header (package/module declaration,
    /// language-pragma lines) and import/include statements derived from
    /// [`Opts::rewrites`].  The CLI populates `rewrites` with the direct
    /// dependencies of the interface before calling this method.
    ///
    /// Returns a heap-allocated [`String`].  The default implementation simply
    /// delegates to [`Self::render_interface`] with no file-level wrapper.
    fn render_file(opts: &Opts<Self>, iface: &Interface) -> String {
        Self::render_interface(opts, iface).to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared Opts struct
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for generating code in language `S`.
///
/// `S` implements [`Syntax`] and encodes all language-specific rendering rules.
///
/// `rewrites` maps 32-byte resource IDs to backend-specific import/include
/// strings for the *direct dependencies* of one interface.  The `pit-gen` CLI
/// populates this map before calling [`Opts::file`].  Semantics per backend:
///
/// | Backend | Value stored in `rewrites` |
/// |---|---|
/// | C | `"P<hex>.h"` (include path) |
/// | Go | `"<module>/p<hex>"` (Go import path; empty = same package) |
/// | Haxe | `"pit.P<hex>"` (FQN; empty = same package) |
/// | TypeScript | `"./P<hex>"` (relative import path, no extension) |
/// | TypeScriptAsync | `"./AP<hex>"` |
/// | Swift | _(unused; all files compiled together)_ |
/// | Haskell | `"P<hex>"` (module name) |
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub struct Opts<S: Syntax> {
    /// Direct-dependency import map.  See the table on [`Opts`] for per-backend
    /// semantics.
    pub rewrites: BTreeMap<[u8; 32], String>,

    /// The [`Syntax`] instance.  Zero-sized for most backends; holds
    /// [`C::prefix`] for the C backend.
    pub syntax: S,
}

impl<S: Syntax> Opts<S> {
    /// Convert a PIT argument type to its target-language representation.
    ///
    /// Returns `impl Display`; call `.to_string()` only when a heap-allocated
    /// [`String`] is strictly required.
    #[inline]
    pub fn ty<'a>(&'a self, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
        S::render_ty(self, arg, this)
    }

    /// Convert a PIT method signature to its target-language representation.
    ///
    /// Returns `impl Display`; call `.to_string()` only when a heap-allocated
    /// [`String`] is strictly required.
    #[inline]
    pub fn meth<'a>(&'a self, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a {
        S::render_meth(self, sig, this)
    }

    /// Convert a PIT interface to a complete target-language declaration.
    ///
    /// Returns `impl Display`; call `.to_string()` only when a heap-allocated
    /// [`String`] is strictly required.
    #[inline]
    pub fn interface<'a>(&'a self, iface: &'a Interface) -> impl Display + 'a {
        S::render_interface(self, iface)
    }

    /// Render a complete source file for a PIT interface.
    ///
    /// Populates `opts.rewrites` with the direct dependencies before calling
    /// this method (see [`Opts`] docs and [`Syntax::render_file`]).
    ///
    /// Returns an owned [`String`].
    #[inline]
    pub fn file(&self, iface: &Interface) -> String {
        S::render_file(self, iface)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// C syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for C.
///
/// Generates [interface99](https://github.com/hirrolot/interface99)-compatible
/// header macros.  No intermediate [`String`] allocations occur — the entire
/// rendering pipeline is `Display`-based via the [`c`] module.
///
/// ## Generated code shape
///
/// ```c
/// #pragma once
/// #include <stdint.h>
/// #include <interface99.h>
/// // (one #include per dependency)
///
/// #define pfx_<hex>_t_IFACE_method(CUR,METH) \
///     vfunc(uint32_t, METH, VSelf, uint64_t p0)
/// #define pfx_<hex>_t_IFACE \
///     pfx_<hex>_t_IFACE_method(pfx_<hex>_t, method)
/// interface(pfx_<hex>_t)
/// ```
///
/// Set [`C::prefix`] (via [`Opts::syntax`]) to customise the type-name prefix:
/// ```ignore
/// let mut opts = Opts::<C>::default();
/// opts.syntax.prefix = "my_".into();
/// println!("{}", opts.file(&iface));
/// ```
#[derive(Default, Clone, Debug)]
pub struct C {
    /// Prefix prepended to every generated C type name.
    pub prefix: String,
}

impl Syntax for C {
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, _this: [u8; 32]) -> impl Display + 'a {
        c::C {
            value: arg,
            kind: c::PureC { cx: opts.syntax.prefix.as_str() },
        }
    }

    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, _this: [u8; 32]) -> impl Display + 'a {
        c::C {
            value: sig,
            kind: c::PureC { cx: opts.syntax.prefix.as_str() },
        }
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
        c::C {
            value: iface,
            kind: c::PureC { cx: opts.syntax.prefix.as_str() },
        }
    }

    fn render_file(opts: &Opts<Self>, iface: &Interface) -> String {
        // opts.rewrites values are include-filenames, e.g. "P<dep_hex>.h"
        let includes: String = opts
            .rewrites
            .values()
            .map(|h| format!("#include \"{h}\"\n"))
            .collect();
        format!(
            "#pragma once\n#include <stdint.h>\n#include <interface99.h>\n{includes}\n{}\n",
            Self::render_interface(opts, iface)
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Go syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for Go.
///
/// All generated files share `package pitbindings`.  Within a single package
/// all interface types are referenced without qualification, so `opts.rewrites`
/// should be left empty for single-package generation (the default used by the
/// CLI).  Cross-package rewrites are supported: set
/// `opts.rewrites[dep_rid] = "mymod/p<hex>"` and the type will be referenced
/// as `p<hex>.P<hex>`.
///
/// ## Generated code shape
///
/// ```go
/// package pitbindings
///
/// type P<hex> interface {
///     P<hex>_method(p0 uint32) uint64
///     P<hex>_multi(p0 float32) (uint32, uint64)
/// }
/// ```
///
/// Borrow flags are ignored.  Go interface values are already implicitly
/// nilable, so the `nullable` flag is not represented separately.
#[derive(Default, Clone, Debug)]
pub struct Go;

impl Syntax for Go {
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
        // Borrow flags (`take`) are ignored — Go has no ownership semantics.
        // Nullable is also ignored — all Go interface types can hold nil.
        match &arg.ty {
            ArgTy::I32 => "uint32".to_string(),
            ArgTy::I64 => "uint64".to_string(),
            ArgTy::F32 => "float32".to_string(),
            ArgTy::F64 => "float64".to_string(),
            ArgTy::Resource { ty, .. } => match ty {
                ResTy::None => "interface{}".to_string(),
                ResTy::Of(id) => match opts.rewrites.get(id) {
                    // Cross-package: "pkgname.TypeName"
                    Some(pkg) => format!("{pkg}.P{}", hex::encode(id)),
                    // Same package: bare type name
                    None => format!("P{}", hex::encode(id)),
                },
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
            .join(", ");
        let ret = match sig.rets.len() {
            0 => String::new(),
            1 => format!(" {}", opts.ty(&sig.rets[0], this)),
            _ => {
                let rets = sig
                    .rets
                    .iter()
                    .map(|a| opts.ty(a, this).to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(" ({rets})")
            }
        };
        format!("({params}){ret}")
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
        let this = iface.rid();
        let hex = hex::encode(this);
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("\tP{hex}_{name}{}", opts.meth(sig, this)))
            .collect::<Vec<_>>()
            .join("\n");
        format!("type P{hex} interface {{\n{methods}\n}}")
    }

    fn render_file(opts: &Opts<Self>, iface: &Interface) -> String {
        let _hex = hex::encode(iface.rid());
        // rewrites values are Go import paths (empty = same-package, no import needed)
        let imports: String = opts
            .rewrites
            .values()
            .map(|pkg| format!("\t\"{pkg}\"\n"))
            .collect();
        let import_block = if imports.is_empty() {
            String::new()
        } else {
            format!("import (\n{imports})\n\n")
        };
        format!(
            "package pitbindings\n\n{import_block}{}\n",
            Self::render_interface(opts, iface)
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Haxe syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for Haxe.
///
/// All generated files live in `package pit`.  Within a single package types
/// are visible without imports; `opts.rewrites` is only needed for
/// cross-package references.
///
/// ## Generated code shape
///
/// ```haxe
/// package pit;
///
/// interface P<hex> {
///   public function p<hex>_method(p0: haxe.Int32): haxe.Int64;
///   public function p<hex>_nullable(p0: haxe.Int32): Null<P<dep_hex>>;
/// }
/// ```
///
/// Borrow flags are ignored.
#[derive(Default, Clone, Debug)]
pub struct Haxe;

impl Syntax for Haxe {
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
        match &arg.ty {
            ArgTy::I32 => "haxe.Int32".to_string(),
            ArgTy::I64 => "haxe.Int64".to_string(),
            ArgTy::F32 => "Float".to_string(),
            ArgTy::F64 => "Float".to_string(),
            ArgTy::Resource { ty, nullable, .. } => {
                // Borrow flags ignored.
                let base = match ty {
                    ResTy::None => "Dynamic".to_string(),
                    ResTy::Of(id) => match opts.rewrites.get(id) {
                        Some(fqn) => fqn.clone(),
                        None => format!("P{}", hex::encode(id)),
                    },
                    ResTy::This => format!("P{}", hex::encode(this)),
                    _ => todo!(),
                };
                if *nullable { format!("Null<{base}>") } else { base }
            }
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
            .join(", ");
        let ret = match sig.rets.len() {
            0 => "Void".to_string(),
            1 => opts.ty(&sig.rets[0], this).to_string(),
            _ => {
                let fields = sig
                    .rets
                    .iter()
                    .enumerate()
                    .map(|(i, a)| format!("r{i}: {}", opts.ty(a, this)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{fields}}}")
            }
        };
        format!("({params}): {ret}")
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
        let this = iface.rid();
        let hex = hex::encode(this);
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| {
                format!("  public function p{hex}_{name}{};", opts.meth(sig, this))
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("interface P{hex} {{\n{methods}\n}}")
    }

    fn render_file(opts: &Opts<Self>, iface: &Interface) -> String {
        // rewrites values are FQNs to import (e.g. "pit.P<dep_hex>")
        let imports: String = opts
            .rewrites
            .values()
            .map(|fqn| format!("import {fqn};\n"))
            .collect();
        format!(
            "package pit;\n\n{imports}\n{}\n",
            Self::render_interface(opts, iface)
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TypeScript (sync) syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for TypeScript (synchronous).
///
/// ## Generated code shape
///
/// ```typescript
/// import type { P<dep_hex> } from "./P<dep_hex>";
///
/// export type P<hex> = {
///   P<hex>_method(p0: number): void;
///   P<hex>_query(p0: number): [bigint];
/// }
/// ```
///
/// Borrow flags are ignored.
/// Nullable resources render as `T | undefined`.
#[derive(Default, Clone, Debug)]
pub struct TypeScript;

impl TypeScript {
    /// Shared type-rendering helper used by both sync and async variants.
    ///
    /// `prefix` is prepended to resource type names (`""` for sync, `"A"` for async).
    pub(crate) fn ty_inner(_opts: &Opts<Self>, arg: &Arg, this: [u8; 32], prefix: &str) -> String {
        match &arg.ty {
            ArgTy::I32 => "number".to_string(),
            ArgTy::I64 => "bigint".to_string(),
            ArgTy::F32 => "number".to_string(),
            ArgTy::F64 => "number".to_string(),
            ArgTy::Resource { ty, nullable, .. } => {
                // Borrow flags ignored.
                let base = match ty {
                    ResTy::None => "any".to_string(),
                    ResTy::Of(id) => format!("{prefix}P{}", hex::encode(id)),
                    ResTy::This => format!("{prefix}P{}", hex::encode(this)),
                    _ => todo!(),
                };
                if *nullable { format!("{base} | undefined") } else { base }
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
            .join(", ");
        let ret = if sig.rets.is_empty() {
            "void".to_string()
        } else {
            let rets = sig
                .rets
                .iter()
                .map(|a| opts.ty(a, this).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{rets}]")
        };
        format!("({params}): {ret}")
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
        let this = iface.rid();
        let hex = hex::encode(this);
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("  P{hex}_{name}{}", opts.meth(sig, this)))
            .collect::<Vec<_>>()
            .join(";\n");
        format!("export type P{hex} = {{\n{methods}\n}}")
    }

    fn render_file(opts: &Opts<Self>, iface: &Interface) -> String {
        // rewrites: dep_rid → relative import path (e.g. "./P<dep_hex>")
        let imports: String = opts
            .rewrites
            .iter()
            .map(|(rid, path)| {
                let hex = hex::encode(rid);
                format!("import type {{ P{hex} }} from \"{path}\";\n")
            })
            .collect();
        format!("{imports}\n{}\n", Self::render_interface(opts, iface))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TypeScript (async) syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for TypeScript with async / Promise support.
///
/// Type names carry the `A` prefix (e.g. `AP<hex>`).
/// Returns include a `| Promise<…>` union alternative.
///
/// ## Generated code shape
///
/// ```typescript
/// import type { AP<dep_hex> } from "./AP<dep_hex>";
///
/// export type AP<hex> = {
///   AP<hex>_method(p0: number): void | Promise<void>;
///   AP<hex>_query(p0: number): [bigint] | Promise<[bigint]>
/// }
/// ```
#[derive(Default, Clone, Debug)]
pub struct TypeScriptAsync;

impl Syntax for TypeScriptAsync {
    fn render_ty<'a>(_opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
        // Build a temporary sync Opts so we can reuse ty_inner.
        // TypeScript's ty_inner does not use opts.rewrites, so an empty Opts is fine.
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
            .join(", ");
        let ret = if sig.rets.is_empty() {
            "void | Promise<void>".to_string()
        } else {
            let rets = sig
                .rets
                .iter()
                .map(|a| opts.ty(a, this).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{rets}] | Promise<[{rets}]>")
        };
        format!("({params}): {ret}")
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
        let this = iface.rid();
        let hex = hex::encode(this);
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("  AP{hex}_{name}{}", opts.meth(sig, this)))
            .collect::<Vec<_>>()
            .join(";\n");
        format!("export type AP{hex} = {{\n{methods}\n}}")
    }

    fn render_file(opts: &Opts<Self>, iface: &Interface) -> String {
        // rewrites: dep_rid → relative import path using "AP" names
        let imports: String = opts
            .rewrites
            .iter()
            .map(|(rid, path)| {
                let hex = hex::encode(rid);
                format!("import type {{ AP{hex} }} from \"{path}\";\n")
            })
            .collect();
        format!("{imports}\n{}\n", Self::render_interface(opts, iface))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Swift syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for Swift.
///
/// Generates standard Swift protocols targeting Swift 5.7+.
/// All generated `.swift` files are compiled together in one module, so
/// no cross-file imports are needed.
///
/// ## Generated code shape
///
/// ```swift
/// public protocol P<hex> {
///     func p<hex>_read(_ p0: UInt32) -> any P<dep_hex>
///     func p<hex>_create() -> Self?
///     func p<hex>_sizes() -> (UInt32, UInt64)
///     func p<hex>_reset()
/// }
/// ```
///
/// Borrow flags are ignored.
/// `ResTy::This` renders as `Self` (the protocol's implicit associated type).
/// Nullable renders as `T?` (Swift optional).
/// Multiple return values render as a Swift tuple `(T1, T2)`.
#[derive(Default, Clone, Debug)]
pub struct Swift;

impl Syntax for Swift {
    fn render_ty<'a>(_opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
        // Borrow flags ignored — Swift has ARC, not affine types.
        let _ = this; // used only for ResTy::This below
        match &arg.ty {
            ArgTy::I32 => "UInt32".to_string(),
            ArgTy::I64 => "UInt64".to_string(),
            ArgTy::F32 => "Float".to_string(),
            ArgTy::F64 => "Double".to_string(),
            ArgTy::Resource { ty, nullable, .. } => {
                let base = match ty {
                    ResTy::None => "Any".to_string(),
                    ResTy::Of(id) => format!("any P{}", hex::encode(id)),
                    // Self is the protocol's own conforming type
                    ResTy::This => "Self".to_string(),
                    _ => todo!(),
                };
                // Swift requires `(any Proto)?` not `any Proto?` for optional existentials
                if *nullable {
                    match &base {
                        b if b.starts_with("any ") => format!("({b})?"),
                        b => format!("{b}?"),
                    }
                } else {
                    base
                }
            }
            _ => todo!(),
        }
    }

    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a {
        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, a)| format!("_ p{i}: {}", opts.ty(a, this)))
            .collect::<Vec<_>>()
            .join(", ");
        let ret = match sig.rets.len() {
            0 => String::new(),
            1 => format!(" -> {}", opts.ty(&sig.rets[0], this)),
            _ => {
                let rets = sig
                    .rets
                    .iter()
                    .map(|a| opts.ty(a, this).to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(" -> ({rets})")
            }
        };
        format!("({params}){ret}")
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
        let this = iface.rid();
        let hex = hex::encode(this);
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("    func p{hex}_{name}{}", opts.meth(sig, this)))
            .collect::<Vec<_>>()
            .join("\n");
        format!("public protocol P{hex} {{\n{methods}\n}}")
    }

    fn render_file(opts: &Opts<Self>, iface: &Interface) -> String {
        // All .swift files in a target are compiled together — no imports needed.
        format!("{}\n", Self::render_interface(opts, iface))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Haskell (monad-generic, associated resource type) syntax
// ─────────────────────────────────────────────────────────────────────────────

/// [`Syntax`] implementation for Haskell — monad-generic with an associated
/// resource type.
///
/// Each PIT interface becomes a Haskell typeclass parameterised over a monad
/// `m`.  The "self" / resource type is an **associated type** `Self m`, not a
/// separate class parameter.  This ensures there is at most one implementation
/// of each interface per monad — a principled design for capability-based APIs.
///
/// Dependency interfaces appear as superclass constraints; their resource types
/// are referenced as `P<dep_hex>.Self m`.
///
/// ## Type mapping
///
/// | PIT type | Haskell type |
/// |---|---|
/// | `i32` | `Word32` |
/// | `i64` | `Word64` |
/// | `f32` | `Float` |
/// | `f64` | `Double` |
/// | `resource(this)` | `Self m` (own associated type) |
/// | `resource(of id)` | `P<hex>.Self m` (dep associated type, qualified) |
/// | `resource(none)` | `()` |
/// | nullable | `Maybe (inner)` |
/// | borrow flag | ignored |
///
/// ## Generated module shape
///
/// ```haskell
/// {-# LANGUAGE TypeFamilies #-}
/// module P<hex> (P<hex>(..)) where
///
/// import Data.Kind (Type)
/// import Data.Word (Word32, Word64)
/// import qualified P<dep_hex> (P<dep_hex>(..))
///
/// class (Monad m, P<dep_hex>.P<dep_hex> m) => P<hex> m where
///   type Self m :: Type
///   p<hex>_read  :: Self m -> Word32 -> m (P<dep_hex>.Self m)
///   p<hex>_write :: Self m -> P<dep_hex>.Self m -> m ()
/// ```
///
/// `opts.rewrites[dep_rid]` = `"P<dep_hex>"` (the module name).
#[derive(Default, Clone, Debug)]
pub struct Haskell;

impl Haskell {
    /// Wrap a Haskell type string in parens when it contains spaces.
    ///
    /// Required when the type appears as an argument to a type constructor
    /// (e.g. `m (Self m)` rather than `m Self m`).
    fn hs_arg(s: &str) -> String {
        if s.contains(' ') { format!("({s})") } else { s.to_string() }
    }

    /// Render a PIT [`Arg`] to a Haskell type string.
    ///
    /// Uses `opts.rewrites` to resolve dep module names.
    fn ty_str(opts: &Opts<Self>, arg: &Arg) -> String {
        match &arg.ty {
            ArgTy::I32 => "Word32".into(),
            ArgTy::I64 => "Word64".into(),
            ArgTy::F32 => "Float".into(),
            ArgTy::F64 => "Double".into(),
            ArgTy::Resource { ty, nullable, .. } => {
                // Borrow flags (take/&) are ignored.
                let base: String = match ty {
                    ResTy::None => "()".into(),
                    ResTy::Of(id) => {
                        let modname = opts
                            .rewrites
                            .get(id)
                            .cloned()
                            .unwrap_or_else(|| format!("P{}", hex::encode(id)));
                        format!("{modname}.Self m")
                    }
                    ResTy::This => "Self m".into(),
                    _ => todo!(),
                };
                if *nullable {
                    format!("Maybe {}", Self::hs_arg(&base))
                } else {
                    base
                }
            }
            _ => todo!(),
        }
    }

    /// Collect the unique set of dependency RIDs referenced in `iface`
    /// (i.e. all `ResTy::Of(id)` values that differ from `iface`'s own RID).
    fn dep_rids(iface: &Interface) -> BTreeMap<[u8; 32], ()> {
        let this = iface.rid();
        let mut seen: BTreeMap<[u8; 32], ()> = BTreeMap::new();
        for sig in iface.methods.values() {
            for arg in sig.params.iter().chain(sig.rets.iter()) {
                if let ArgTy::Resource { ty: ResTy::Of(id), .. } = &arg.ty {
                    if *id != this {
                        seen.insert(*id, ());
                    }
                }
            }
        }
        seen
    }
}

impl Syntax for Haskell {
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, _this: [u8; 32]) -> impl Display + 'a {
        Self::ty_str(opts, arg)
    }

    /// Renders the full type annotation for a method:
    /// `:: Self m -> p0_ty -> … -> m ret_ty`
    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a {
        let params: Vec<String> = sig
            .params
            .iter()
            .map(|a| opts.ty(a, this).to_string())
            .collect();
        let rets: Vec<String> = sig
            .rets
            .iter()
            .map(|a| opts.ty(a, this).to_string())
            .collect();

        // Monadic return type — all results wrapped in m (...)
        // 0 rets → m ()
        // 1 ret  → m T  or  m (Complex T)  (parens when type has spaces)
        // N rets → m (T1, T2, …)
        let monadic_ret = match rets.len() {
            0 => "m ()".to_string(),
            1 => format!("m {}", Self::hs_arg(&rets[0])),
            _ => format!("m ({})", rets.join(", ")),
        };

        // Full arrow chain: Self m → p0 → … → m (…)
        let mut arrows = Vec::new();
        arrows.push("Self m".to_string());
        arrows.extend(params);
        arrows.push(monadic_ret);
        format!(":: {}", arrows.join(" -> "))
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
        let this = iface.rid();
        let hex = hex::encode(this);

        // Collect dep module names for superclass constraints
        let dep_rids = Self::dep_rids(iface);
        let mut constraints = Vec::new();
        constraints.push("Monad m".to_string());
        for id in dep_rids.keys() {
            let modname = opts
                .rewrites
                .get(id)
                .cloned()
                .unwrap_or_else(|| format!("P{}", hex::encode(id)));
            // Qualified class name: ModuleName.ClassName m
            constraints.push(format!("{modname}.{modname} m"));
        }
        let ctx = if constraints.len() == 1 {
            constraints.remove(0)
        } else {
            format!("({})", constraints.join(", "))
        };

        let methods: Vec<String> = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("  p{hex}_{name} {}", opts.meth(sig, this)))
            .collect();

        format!(
            "class {ctx} => P{hex} m where\n  type Self m :: Type\n{}",
            methods.join("\n")
        )
    }

    fn render_file(opts: &Opts<Self>, iface: &Interface) -> String {
        let hex = hex::encode(iface.rid());
        let dep_rids = Self::dep_rids(iface);

        // Import each dep module qualified, bringing the class and associated type into scope
        let imports: String = dep_rids
            .keys()
            .map(|id| {
                let modname = opts
                    .rewrites
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| format!("P{}", hex::encode(id)));
                format!("import qualified {modname} ({modname}(..))\n")
            })
            .collect();

        format!(
            "{{-# LANGUAGE TypeFamilies #-}}\n\
             module P{hex} (P{hex}(..)) where\n\
             \n\
             import Data.Kind (Type)\n\
             import Data.Word (Word32, Word64)\n\
             {imports}\
             \n\
             {}\n",
            Self::render_interface(opts, iface)
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Java syntax
// ─────────────────────────────────────────────────────────────────────────────

/// Pure Java interface (`P<hex>`) for more-pit canonical definitions.
#[derive(Default, Clone, Debug)]
pub struct Java;

impl Syntax for Java {
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
        match &arg.ty {
            ArgTy::I32 => "int".to_string(),
            ArgTy::I64 => "long".to_string(),
            ArgTy::F32 => "float".to_string(),
            ArgTy::F64 => "double".to_string(),
            ArgTy::Resource { ty, nullable, .. } => {
                let base = match ty {
                    ResTy::None => "Object".to_string(),
                    ResTy::Of(id) => format!("P{}", hex::encode(id)),
                    ResTy::This => format!("P{}", hex::encode(this)),
                    _ => todo!(),
                };
                if *nullable {
                    format!("{base} | null")
                } else {
                    base
                }
            }
            _ => todo!(),
        }
    }

    fn render_meth<'a>(opts: &'a Opts<Self>, sig: &'a Sig, this: [u8; 32]) -> impl Display + 'a {
        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, a)| format!("{} p{i}", opts.ty(a, this)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("({params})")
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
        let hex = hex::encode(iface.rid());
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| {
                let ret = if sig.rets.is_empty() {
                    "void".to_string()
                } else if sig.rets.len() == 1 {
                    opts.ty(&sig.rets[0], iface.rid()).to_string()
                } else {
                    "Object".to_string()
                };
                format!("  {ret} {name}{}", opts.meth(sig, iface.rid()))
            })
            .collect::<Vec<_>>()
            .join(";\n");
        format!("public interface P{hex} {{\n{methods};\n}}")
    }

    fn render_file(opts: &Opts<Self>, iface: &Interface) -> String {
        let pkg = "pc.portal.pit.guest";
        format!(
            "package {pkg};\n\n{}\n",
            Self::render_interface(opts, iface)
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scala syntax
// ─────────────────────────────────────────────────────────────────────────────

/// Pure Scala trait (`P<hex>`) for more-pit canonical definitions.
#[derive(Default, Clone, Debug)]
pub struct Scala;

impl Syntax for Scala {
    fn render_ty<'a>(opts: &'a Opts<Self>, arg: &'a Arg, this: [u8; 32]) -> impl Display + 'a {
        match &arg.ty {
            ArgTy::I32 => "Int".to_string(),
            ArgTy::I64 => "Long".to_string(),
            ArgTy::F32 => "Float".to_string(),
            ArgTy::F64 => "Double".to_string(),
            ArgTy::Resource { ty, nullable, .. } => {
                let base = match ty {
                    ResTy::None => "Any".to_string(),
                    ResTy::Of(id) => format!("P{}", hex::encode(id)),
                    ResTy::This => format!("P{}", hex::encode(this)),
                    _ => todo!(),
                };
                if *nullable {
                    format!("Option[{base}]")
                } else {
                    base
                }
            }
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
            .join(", ");
        let ret = if sig.rets.is_empty() {
            "Unit".to_string()
        } else if sig.rets.len() == 1 {
            opts.ty(&sig.rets[0], this).to_string()
        } else {
            format!(
                "({})",
                sig.rets
                    .iter()
                    .map(|a| opts.ty(a, this).to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        format!("({params}): {ret}")
    }

    fn render_interface<'a>(opts: &'a Opts<Self>, iface: &'a Interface) -> impl Display + 'a {
        let hex = hex::encode(iface.rid());
        let methods = iface
            .methods
            .iter()
            .map(|(name, sig)| format!("  def {name}{}", opts.meth(sig, iface.rid())))
            .collect::<Vec<_>>()
            .join("\n");
        format!("trait P{hex} {{\n{methods}\n}}")
    }

    fn render_file(opts: &Opts<Self>, iface: &Interface) -> String {
        let pkg = "pc.portal.pit.guest.scala";
        format!(
            "package {pkg}\n\n{}\n",
            Self::render_interface(opts, iface)
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience type aliases matching the old per-crate public API
// ─────────────────────────────────────────────────────────────────────────────

/// Type alias for C code generation options.
pub type COpts = Opts<C>;
/// Type alias for Go code generation options.
pub type GoOpts = Opts<Go>;
/// Type alias for Haxe code generation options.
pub type HaxeOpts = Opts<Haxe>;
/// Type alias for TypeScript (sync) code generation options.
pub type TsOpts = Opts<TypeScript>;
/// Type alias for TypeScript (async) code generation options.
pub type TsOptsAsync = Opts<TypeScriptAsync>;
/// Type alias for Swift code generation options.
pub type SwiftOpts = Opts<Swift>;
/// Type alias for Haskell (monad-generic) code generation options.
pub type HaskellOpts = Opts<Haskell>;
