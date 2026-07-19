//! # pit-go-generic
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
//! pit-go-generic = { … }
//!
//! # After
//! pit-lang-generic = { … }
//! ```
//!
//! ```ignore
//! // Before
//! use pit_go_generic::GoOpts;
//!
//! // After
//! use pit_lang_generic::{Opts, Go};      // or: use pit_lang_generic::GoOpts;
//! ```

#![no_std]

pub use pit_lang_generic::{Go, GoOpts, Opts, Syntax};
