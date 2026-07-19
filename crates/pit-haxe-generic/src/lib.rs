//! # pit-haxe-generic
//!
//! > **This crate is now a thin re-export of [`pit_lang_generic`].**
//! >
//! > All types and functions live in `pit_lang_generic`; this crate exists
//! > for backwards compatibility only.  Please migrate to `pit-lang-generic`.
//!
//! ## Migration
//!
//! ```toml
//! # Before
//! pit-haxe-generic = { … }
//!
//! # After
//! pit-lang-generic = { … }
//! ```
//!
//! ```ignore
//! // Before
//! use pit_haxe_generic::HaxeOpts;
//!
//! // After
//! use pit_lang_generic::{Opts, Haxe};      // or: use pit_lang_generic::HaxeOpts;
//! ```

#![no_std]

pub use pit_lang_generic::{Haxe, HaxeOpts, Opts, Syntax};
