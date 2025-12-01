//! # pit-ts-generic
//!
//! Code generator for TypeScript type definitions from PIT (Portal Interface Types) interfaces.
//!
//! This `no_std` compatible crate generates TypeScript types from PIT interface definitions.
//! It supports both synchronous and asynchronous (Promise-based) type generation.
//!
//! ## Overview
//!
//! The main type is [`TsOpts`] which provides methods for generating:
//! - [`TsOpts::interface`] - Complete TypeScript type definition
//! - [`TsOpts::meth`] - Method signature
//! - [`TsOpts::ty`] - Type expression
//!
//! ## Example
//!
//! ```ignore
//! use pit_ts_generic::TsOpts;
//! use pit_core::Interface;
//!
//! let iface: Interface = /* parse from PIT format */;
//! let opts = TsOpts { r#async: true };
//! let ts_code = opts.interface(&iface);
//! println!("{}", ts_code);
//! ```
//!
//! ## Async Support
//!
//! When `async` is enabled, the generated types include `| Promise<[...]>` return
//! types and type names are prefixed with `A` (e.g., `AP<hex_id>`).
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

/// Configuration options for TypeScript code generation.
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub struct TsOpts {
    /// Enable async/Promise support in generated types.
    ///
    /// When `true`:
    /// - Type names are prefixed with `A` (e.g., `AP<hex_id>`)
    /// - Return types include `| Promise<[...]>` variant
    pub r#async: bool,
    // pub rewrites: BTreeMap<[u8; 32], String>,
}
impl TsOpts {
    /// Converts a PIT argument type to its TypeScript type representation.
    ///
    /// # Arguments
    ///
    /// * `t` - The PIT argument type
    /// * `this` - The resource ID of the containing interface
    ///
    /// # Returns
    ///
    /// A string containing the TypeScript type (e.g., `number`, `bigint`, `any`, `P<hex_id>`).
    /// Nullable types are rendered as `T | undefined`.
    pub fn ty(&self, t: &Arg, this: [u8; 32]) -> String {
        let m = match self.r#async {
            true => "A",
            false => "",
        };
        match t {
            Arg::I32 => format!("number"),
            Arg::I64 => format!("bigint"),
            Arg::F32 => format!("number"),
            Arg::F64 => format!("number"),
            Arg::Resource {
                ty,
                nullable,
                take,
                ann,
            } => match match ty {
                pit_core::ResTy::None => format!("any"),
                pit_core::ResTy::Of(a) => format!("{m}P{}", hex::encode(a)),
                pit_core::ResTy::This => format!("{m}P{}", hex::encode(this)),
                _ => todo!(),
            } {
                ty => {
                    if *nullable {
                        format!("{ty} | undefined")
                    } else {
                        ty
                    }
                }
            },
            _ => todo!(),
        }
    }

    /// Generates a TypeScript method signature from a PIT method signature.
    ///
    /// # Arguments
    ///
    /// * `s` - The PIT method signature
    /// * `this` - The resource ID of the containing interface
    ///
    /// # Returns
    ///
    /// A string containing the TypeScript method signature.
    /// When async mode is enabled, includes `| Promise<[...]>` in the return type.
    pub fn meth(&self, s: &Sig, this: [u8; 32]) -> String {
        let j = s
            .rets
            .iter()
            .map(|x| self.ty(x, this))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "({}): [{}]{}",
            s.params
                .iter()
                .enumerate()
                .map(|(a, b)| format!("p{a}: {}", self.ty(b, this)))
                .collect::<Vec<_>>()
                .join(","),
            j,
            if self.r#async {
                format!("| Promise<[{j}]>")
            } else {
                String::default()
            }
        )
    }

    /// Generates a complete TypeScript type definition from a PIT interface.
    ///
    /// This is the main entry point for generating TypeScript code from PIT interfaces.
    /// The generated type name is `P<hex_id>` (or `AP<hex_id>` in async mode).
    ///
    /// # Arguments
    ///
    /// * `i` - The PIT interface to convert
    ///
    /// # Returns
    ///
    /// A string containing the complete TypeScript type definition.
    ///
    /// # Example Output
    ///
    /// ```typescript
    /// export type P<hex_id> = {P<hex_id>_methodName (p0: number): [number]}
    /// ```
    pub fn interface(&self, i: &Interface) -> String {
        let this = i.rid();
        let m = match self.r#async {
            true => "A",
            false => "",
        };
        format!(
            "export type {m}P{} = {{{}}}",
            hex::encode(this),
            i.methods
                .iter()
                .map(|(a, b)| format!("{m}P{}_{a} {}", hex::encode(this), self.meth(b, this)))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}
