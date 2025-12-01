//! # pit-haxe-generic
//!
//! Code generator for Haxe interface definitions from PIT (Portal Interface Types) interfaces.
//!
//! This `no_std` compatible crate generates Haxe interfaces from PIT interface definitions.
//! The generated Haxe code uses standard Haxe interface syntax.
//!
//! ## Overview
//!
//! The main type is [`HaxeOpts`] which provides methods for generating:
//! - [`HaxeOpts::interface`] - Complete Haxe interface definition
//! - [`HaxeOpts::meth`] - Method signature
//! - [`HaxeOpts::ty`] - Type expression
//!
//! ## Example
//!
//! ```ignore
//! use pit_haxe_generic::HaxeOpts;
//! use pit_core::Interface;
//!
//! let iface: Interface = /* parse from PIT format */;
//! let opts = HaxeOpts::default();
//! let haxe_code = opts.interface(&iface);
//! println!("{}", haxe_code);
//! ```
//!
//! ## Features
//!
//! - `unstable-sdk` - Enable portal-solutions-sdk integration
//! - `unstable-pcode` - Enable pcode expression support
//! - `unstable-generics` - Enable generic parameter support

#![no_std]
use alloc::{collections::btree_map::BTreeMap, format, string::String, vec::Vec};
use pit_core::{Arg, Interface, Sig};
extern crate alloc;

/// Configuration options for Haxe code generation.
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub struct HaxeOpts {
    /// Package rewrites for cross-package references.
    ///
    /// Maps 32-byte resource IDs to Haxe package paths.
    /// When a resource type references another interface, this map
    /// determines which package to import it from.
    pub rewrites: BTreeMap<[u8; 32], String>,
}
impl HaxeOpts {
    /// Converts a PIT argument type to its Haxe type representation.
    ///
    /// # Arguments
    ///
    /// * `t` - The PIT argument type
    /// * `this` - The resource ID of the containing interface
    ///
    /// # Returns
    ///
    /// A string containing the Haxe type (e.g., `haxe.Int32`, `Float`, `Dynamic`, `P<hex_id>`).
    pub fn ty(&self, t: &Arg, this: [u8; 32]) -> String {
        match t {
            Arg::I32 => format!("haxe.Int32"),
            Arg::I64 => format!("haxe.Int32"),
            Arg::F32 => format!("Float"),
            Arg::F64 => format!("Float"),
            Arg::Resource {
                ty,
                nullable,
                take,
                ann,
            } => match ty {
                pit_core::ResTy::None => format!("Dynamic"),
                pit_core::ResTy::Of(a) => format!(
                    "{}.P{}",
                    match self.rewrites.get(a) {
                        None => format!("pit{}", hex::encode(a)),
                        Some(b) => b.clone(),
                    },
                    hex::encode(a)
                ),
                pit_core::ResTy::This => format!("P{}", hex::encode(this)),
                _ => todo!(),
            },
            _ => todo!(),
        }
    }

    /// Generates a Haxe method signature from a PIT method signature.
    ///
    /// # Arguments
    ///
    /// * `s` - The PIT method signature
    /// * `this` - The resource ID of the containing interface
    ///
    /// # Returns
    ///
    /// A string containing the Haxe method signature.
    pub fn meth(&self, s: &Sig, this: [u8; 32]) -> String {
        format!(
            "({}): {{{}}}",
            s.params
                .iter()
                .enumerate()
                .map(|(a, b)| format!("p{a}: {}", self.ty(b, this)))
                .collect::<Vec<_>>()
                .join(","),
            s.rets
                .iter()
                .map(|x| self.ty(x, this))
                .enumerate()
                .map(|(a, b)| format!("r{a}: {b}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    /// Generates a complete Haxe interface definition from a PIT interface.
    ///
    /// This is the main entry point for generating Haxe code from PIT interfaces.
    /// The generated interface name is `P<hex_id>` where `hex_id` is the
    /// hex-encoded 32-byte resource ID.
    ///
    /// # Arguments
    ///
    /// * `i` - The PIT interface to convert
    ///
    /// # Returns
    ///
    /// A string containing the complete Haxe interface definition.
    ///
    /// # Example Output
    ///
    /// ```haxe
    /// interface P<hex_id> {P<hex_id>_methodName (p0: haxe.Int32): {r0: Float}}
    /// ```
    pub fn interface(&self, i: &Interface) -> String {
        let this = i.rid();
        format!(
            "interface P{} {{{}}}",
            hex::encode(this),
            i.methods
                .iter()
                .map(|(a, b)| format!("P{}_{a} {}", hex::encode(this), self.meth(b, this)))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}
