//! # pit-go-generic
//!
//! Code generator for Go interface definitions from PIT (Portal Interface Types) interfaces.
//!
//! This `no_std` compatible crate generates Go interfaces from PIT interface definitions.
//! The generated Go code uses standard Go idioms for interface declaration.
//!
//! ## Overview
//!
//! The main type is [`GoOpts`] which provides methods for generating:
//! - [`GoOpts::interface`] - Complete Go interface definition
//! - [`GoOpts::meth`] - Method signature
//! - [`GoOpts::ty`] - Type expression
//!
//! ## Example
//!
//! ```ignore
//! use pit_go_generic::GoOpts;
//! use pit_core::Interface;
//!
//! let iface: Interface = /* parse from PIT format */;
//! let opts = GoOpts::default();
//! let go_code = opts.interface(&iface);
//! println!("{}", go_code);
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

/// Configuration options for Go code generation.
///
/// Controls how Go interfaces and types are generated from PIT interfaces.
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub struct GoOpts {
    /// Package rewrites for cross-package references.
    ///
    /// Maps 32-byte resource IDs to Go package import paths.
    /// When a resource type references another interface, this map
    /// determines which package to import it from.
    pub rewrites: BTreeMap<[u8; 32], String>,
}
impl GoOpts {
    /// Converts a PIT argument type to its Go type representation.
    ///
    /// # Arguments
    ///
    /// * `t` - The PIT argument type
    /// * `this` - The resource ID of the containing interface
    ///
    /// # Returns
    ///
    /// A string containing the Go type (e.g., `uint32`, `interface{}`, `P<hex_id>`).
    pub fn ty(&self, t: &Arg, this: [u8; 32]) -> String {
        match t {
            Arg::I32 => format!("uint32"),
            Arg::I64 => format!("uint64"),
            Arg::F32 => format!("float32"),
            Arg::F64 => format!("float64"),
            Arg::Resource {
                ty,
                nullable,
                take,
                ann,
            } => match ty {
                pit_core::ResTy::None => format!("interface{{}}"),
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

    /// Generates a Go method signature from a PIT method signature.
    ///
    /// # Arguments
    ///
    /// * `s` - The PIT method signature
    /// * `this` - The resource ID of the containing interface
    ///
    /// # Returns
    ///
    /// A string containing the Go method signature (e.g., `(p0 uint32) (uint64)`).
    pub fn meth(&self, s: &Sig, this: [u8; 32]) -> String {
        format!(
            "({}) ({})",
            s.params
                .iter()
                .enumerate()
                .map(|(a, b)| format!("p{a} {}", self.ty(b, this)))
                .collect::<Vec<_>>()
                .join(","),
            s.rets
                .iter()
                .map(|x| self.ty(x, this))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    /// Generates a complete Go interface definition from a PIT interface.
    ///
    /// This is the main entry point for generating Go code from PIT interfaces.
    /// The generated interface name is `P<hex_id>` where `hex_id` is the
    /// hex-encoded 32-byte resource ID.
    ///
    /// # Arguments
    ///
    /// * `i` - The PIT interface to convert
    ///
    /// # Returns
    ///
    /// A string containing the complete Go interface definition.
    ///
    /// # Example Output
    ///
    /// ```go
    /// type P<hex_id> interface{P<hex_id>_methodName (p0 uint32) (uint64)}
    /// ```
    pub fn interface(&self, i: &Interface) -> String {
        let this = i.rid();
        format!(
            "type P{} interface{{{}}}",
            hex::encode(this),
            i.methods
                .iter()
                .map(|(a, b)| format!("P{}_{a} {}", hex::encode(this), self.meth(b, this)))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}
