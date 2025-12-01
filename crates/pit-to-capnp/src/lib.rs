//! # pit-to-capnp
//!
//! Converter from PIT (Portal Interface Types) to Cap'n Proto schema format.
//!
//! This `no_std` compatible crate provides traits and utilities for converting
//! PIT interfaces to Cap'n Proto (capnproto) schema definitions.
//!
//! ## Overview
//!
//! The main types are:
//! - [`Capnp`] - A trait for types that can render themselves as Cap'n Proto schema
//! - [`ViaCapnp`] - A wrapper that implements `Display` via the `Capnp` trait
//!
//! ## Example
//!
//! ```ignore
//! use pit_to_capnp::{Capnp, ViaCapnp};
//! use std::fmt::{Formatter, Result};
//!
//! struct MyType;
//!
//! impl Capnp for MyType {
//!     fn capnp(&self, f: &mut Formatter<'_>) -> Result {
//!         write!(f, "# Cap'n Proto schema")
//!     }
//! }
//!
//! let my_type = MyType;
//! println!("{}", ViaCapnp(&my_type));
//! ```
//!
//! ## Features
//!
//! - `unstable-sdk` - Enable portal-solutions-sdk integration
//! - `unstable-pcode` - Enable pcode expression support
//! - `unstable-generics` - Enable generic parameter support

#![no_std]
use core::fmt::{Display, Formatter};
extern crate alloc;

/// Trait for types that can render themselves as Cap'n Proto schema.
///
/// Implementors should write valid Cap'n Proto schema syntax to the formatter.
pub trait Capnp {
    /// Renders this type as Cap'n Proto schema format.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn capnp(&self, f: &mut Formatter<'_>) -> core::fmt::Result;
}

/// A wrapper that implements `Display` by delegating to [`Capnp::capnp`].
///
/// This allows any type implementing `Capnp` to be used with standard
/// formatting macros like `format!`, `println!`, etc.
#[derive(Clone, Copy)]
pub struct ViaCapnp<'a>(pub &'a (dyn Capnp + 'a));

impl Display for ViaCapnp<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.0.capnp(f)
    }
}
