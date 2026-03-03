//! # pit-swift-generic
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
//! pit-swift-generic = { … }
//!
//! # After
//! pit-lang-generic = { … }
//! ```
//!
//! ```ignore
//! // Before
//! use pit_swift_generic::SwiftOpts;   // or TsOpts (legacy alias)
//!
//! // After
//! use pit_lang_generic::{Opts, Swift};   // or: use pit_lang_generic::SwiftOpts;
//! ```

#![no_std]

pub use pit_lang_generic::{Opts, Swift, SwiftOpts, Syntax};

/// Legacy type alias kept for backwards compatibility.
#[deprecated(since = "0.1.0", note = "Use `SwiftOpts` / `pit_lang_generic::SwiftOpts` instead")]
pub type TsOpts = SwiftOpts;
