//! # pit-swift-generic
//!
//! Code generator for Swift protocol definitions from PIT (Portal Interface Types) interfaces.
//!
//! This `no_std` compatible crate generates Swift protocols from PIT interface definitions.
//! The generated Swift code uses existential types (`any Protocol`) for type-erased references.
//!
//! ## Overview
//!
//! The main type is [`SwiftOpts`] which provides methods for generating:
//! - [`SwiftOpts::interface`] - Complete Swift protocol definition
//! - [`SwiftOpts::meth`] - Method signature
//! - [`SwiftOpts::ty`] - Type expression
//!
//! ## Example
//!
//! ```ignore
//! use pit_swift_generic::SwiftOpts;
//! use pit_core::Interface;
//!
//! let iface: Interface = /* parse from PIT format */;
//! let opts = SwiftOpts::default();
//! let swift_code = opts.interface(&iface);
//! println!("{}", swift_code);
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

/// Type alias for backwards compatibility.
pub type TsOpts = SwiftOpts;

/// Configuration options for Swift code generation.
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub struct SwiftOpts {
    // pub rewrites: BTreeMap<[u8; 32], String>,
}
impl SwiftOpts {
    /// Converts a PIT argument type to its Swift type representation.
    ///
    /// # Arguments
    ///
    /// * `t` - The PIT argument type
    /// * `this` - The resource ID of the containing interface
    ///
    /// # Returns
    ///
    /// A string containing the Swift type (e.g., `UInt32`, `Double`, `Any`, `any P<hex_id>`).
    pub fn ty(&self, t: &Arg, this: [u8; 32]) -> String {
        match t {
            Arg::I32 => format!("UInt32"),
            Arg::I64 => format!("UInt64"),
            Arg::F32 => format!("Float"),
            Arg::F64 => format!("Double"),
            Arg::Resource {
                ty,
                nullable,
                take,
                ann,
            } => match ty {
                pit_core::ResTy::None => format!("Any"),
                pit_core::ResTy::Of(a) => format!("any P{}", hex::encode(a)),
                pit_core::ResTy::This => format!("any P{}", hex::encode(this)),
                _ => todo!(),
            },
            _ => todo!(),
        }
    }

    /// Generates a Swift method signature from a PIT method signature.
    ///
    /// # Arguments
    ///
    /// * `s` - The PIT method signature
    /// * `this` - The resource ID of the containing interface
    ///
    /// # Returns
    ///
    /// A string containing the Swift method signature.
    pub fn meth(&self, s: &Sig, this: [u8; 32]) -> String {
        format!(
            "({}) -> ({})",
            s.params
                .iter()
                .enumerate()
                .map(|(a, b)| format!("p{a} _: {}", self.ty(b, this)))
                .collect::<Vec<_>>()
                .join(","),
            s.rets
                .iter()
                .enumerate()
                .map(|(a, b)| format!("r{a} _: {}", self.ty(b, this)))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    /// Generates a complete Swift protocol definition from a PIT interface.
    ///
    /// This is the main entry point for generating Swift code from PIT interfaces.
    /// The generated protocol name is `P<hex_id>` where `hex_id` is the
    /// hex-encoded 32-byte resource ID.
    ///
    /// # Arguments
    ///
    /// * `i` - The PIT interface to convert
    ///
    /// # Returns
    ///
    /// A string containing the complete Swift protocol definition.
    ///
    /// # Example Output
    ///
    /// ```swift
    /// open protocol P<hex_id> {open P<hex_id>_methodName (p0 _: UInt32) -> (r0 _: UInt64)}
    /// ```
    pub fn interface(&self, i: &Interface) -> String {
        let this = i.rid();
        format!(
            "open protocol P{} {{{}}}",
            hex::encode(this),
            i.methods
                .iter()
                .map(|(a, b)| format!("open P{}_{a} {}", hex::encode(this), self.meth(b, this)))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}
