//! # pit-wit-bridge
//!
//! Bridge from PIT (Portal Interface Types) to WebAssembly Interface Types (WIT) format.
//!
//! This `no_std` compatible crate provides traits and utilities for converting
//! PIT interfaces to WIT (WebAssembly Interface Types) format, enabling
//! integration with the WebAssembly Component Model.
//!
//! ## Overview
//!
//! The main types are:
//! - [`ToWIT`] - A trait for types that can render themselves as WIT format
//! - [`ViaWIT`] - A wrapper that implements `Display` via the `ToWIT` trait
//!
//! ## Example
//!
//! ```ignore
//! use pit_wit_bridge::{ToWIT, ViaWIT};
//! use std::fmt::{Formatter, Result};
//!
//! struct MyInterface;
//!
//! impl ToWIT for MyInterface {
//!     fn to_wit(&self, f: &mut Formatter<'_>) -> Result {
//!         write!(f, "interface my-interface {{ }}")
//!     }
//! }
//!
//! let iface = MyInterface;
//! println!("{}", ViaWIT(&iface));
//! ```
//!
//! ## Features
//!
//! - `unstable-sdk` - Enable portal-solutions-sdk integration
//! - `unstable-pcode` - Enable pcode expression support
//! - `unstable-generics` - Enable generic parameter support

#![no_std]
extern crate alloc;
use core::fmt::{Display, Formatter};

/// Trait for types that can render themselves as WIT (WebAssembly Interface Types) format.
///
/// Implementors should write valid WIT syntax to the formatter.
pub trait ToWIT {
    /// Renders this type as WIT format.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn to_wit(&self, f: &mut Formatter<'_>) -> core::fmt::Result;
}

/// A wrapper that implements `Display` by delegating to [`ToWIT::to_wit`].
///
/// This allows any type implementing `ToWIT` to be used with standard
/// formatting macros like `format!`, `println!`, etc.
#[derive(Clone, Copy)]
pub struct ViaWIT<'a>(pub &'a (dyn ToWIT + 'a));

impl Display for ViaWIT<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.0.to_wit(f)
    }
}
